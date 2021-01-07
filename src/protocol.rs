use std::io;

use crate::{controls::{Control, RawControl}, controls_impl::parse_bind_response};
use crate::controls_impl::{build_tag, parse_controls};
use crate::search::SearchItem;
use crate::RequestId;

use lber::common::TagClass;
use lber::parse::parse_uint;
use lber::parse::Parser;
use lber::structure::{StructureTag, PL};
use lber::structures::{ASNTag, Integer, Sequence, Tag};
use lber::universal::Types;
use lber::write;
use lber::{Consumer, ConsumerState, IResult, Input, Move};

use bytes::{Buf, BytesMut};
use tokio::sync::{mpsc, oneshot};
use tokio_util::codec::{Decoder, Encoder};

pub struct LdapCodec;

pub(crate) type MaybeControls = Option<Vec<RawControl>>;
pub(crate) type ItemSender = mpsc::UnboundedSender<(SearchItem, Vec<Control>)>;
pub(crate) type ResultSender = oneshot::Sender<(Tag, Vec<Control>)>;

#[derive(Debug)]
pub enum LdapOp {
    Single,
    Search(ItemSender),
    Abandon(RequestId),
    Unbind,
}

impl Decoder for LdapCodec {
    type Item = (RequestId, (Tag, Vec<Control>));
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let decoding_error = io::Error::new(io::ErrorKind::Other, "decoding error");
        let mut parser = Parser::new();
        let (amt, tag) = match *parser.handle(Input::Element(buf)) {
            ConsumerState::Continue(_) => return Ok(None),
            ConsumerState::Error(_e) => return Err(decoding_error),
            ConsumerState::Done(amt, ref tag) => (amt, tag),
        };
        let amt = match amt {
            Move::Await(_) => return Ok(None),
            Move::Seek(_) => return Err(decoding_error),
            Move::Consume(amt) => amt,
        };
        buf.advance(amt);
        let tag = tag.clone();
        let mut tags = match tag
            .match_id(Types::Sequence as u64)
            .and_then(|t| t.expect_constructed())
        {
            Some(tags) => tags,
            None => return Err(decoding_error),
        };
        let mut maybe_controls = tags.pop().expect("element");
        let has_controls = match maybe_controls {
            StructureTag {
                id,
                class,
                ref payload,
            } if class == TagClass::Context && id == 0 => match *payload {
                PL::C(_) => true,
                PL::P(_) => return Err(decoding_error),
            },
            StructureTag { id, class, .. } if class == TagClass::Context && id == 10 => {
                // Active Directory bug workaround
                //
                // AD incorrectly encodes Notice of Disconnection messages. The OID of the
                // Unsolicited Notification should be part of the ExtendedResponse sequence
                // but AD puts it outside, where the optional controls belong. This confuses
                // our parser, which doesn't expect the extra sequence element at the end
                // and crashes. This match arm thus ignores the element.
                maybe_controls = tags.pop().expect("element");
                false
            }
            _ => false,
        };
        let (protoop, controls) = if has_controls {
            (tags.pop().expect("element"), Some(maybe_controls))
        } else {
            (maybe_controls, None)
        };
        let controls = match controls {
            Some(controls) => parse_controls(controls),
            None => vec![],
        };
        let msgid = match parse_uint(
            tags.pop()
                .expect("element")
                .match_class(TagClass::Universal)
                .and_then(|t| t.match_id(Types::Integer as u64))
                .and_then(|t| t.expect_primitive())
                .expect("message id")
                .as_slice(),
        ) {
            IResult::Done(_, id) => id as i32,
            _ => return Err(decoding_error),
        };
//        Ok(Some((msgid, (Tag::StructureTag(protoop), controls))))

        match protoop.id {
            1 => {
                let controls = parse_bind_response(protoop.clone());
                Ok(Some((msgid, (Tag::StructureTag(protoop), controls))))
            }
            _ => Ok(Some((msgid, (Tag::StructureTag(protoop), controls)))),
        }
    }
}

impl Encoder<(RequestId, Tag, MaybeControls)> for LdapCodec {
    type Error = io::Error;

    fn encode(
        &mut self,
        msg: (RequestId, Tag, MaybeControls),
        into: &mut BytesMut,
    ) -> io::Result<()> {
        let (id, tag, controls) = msg;
        let outstruct = {
            let mut msg = vec![
                Tag::Integer(Integer {
                    inner: id as i64,
                    ..Default::default()
                }),
                tag,
            ];
            if let Some(controls) = controls {
                msg.push(Tag::StructureTag(StructureTag {
                    id: 0,
                    class: TagClass::Context,
                    payload: PL::C(controls.into_iter().map(build_tag).collect()),
                }));
            }
            Tag::Sequence(Sequence {
                inner: msg,
                ..Default::default()
            })
            .into_structure()
        };
        write::encode_into(into, outstruct)?;
        Ok(())
    }
}
