# LDAP client library

A pure-Rust LDAP client library using the Tokio stack.

### Interaction with Tokio versions

Tokio reached 1.0 rather soon after 0.3, but it's not a 1:1 replacement. Furthermore,
the bump of the `bytes` crate to 1.0 made it necessary to increase the `lber` version
number. For that reason, `ldap3` will also go one version up, to 0.9. Things should
be quieter from now on because of Tokio's 1.0 compatibility promise, though.

All functional changes in 0.9.1 have been backported to 0.8.3 (Tokio 0.3) and 0.7.4
(Tokio 0.2). The plan is to limit further 0.8.x and 0.7.x changes to bug- and
compatibility fixes for the next six months, and continue development solely on 0.9.x.

### Documentation

- [Version 0.9.x (Tokio 1.0)](https://docs.rs/ldap3/0.9.1/ldap3/)

- [Version 0.8.x (Tokio 0.3)](https://docs.rs/ldap3/0.8.3/ldap3/)

- [Version 0.7.x (Tokio 0.2)](https://docs.rs/ldap3/0.7.4/ldap3/)

### Note

The library is client-only. One cannot make an LDAP server or a proxy with it.

## Usage

The library can be used either synchronously or asynchronously. The aim is to
offer essentially the same call interface for both flavors, with the necessary
differences in interaction and return values according to the nature of I/O.

Add this to your `Cargo.toml`:

```toml
[dependencies.ldap3]
version = "0.9"
```

## Examples

The following two examples perform exactly the same operation and should produce identical
results. They should be run against the example server in the `data` subdirectory of the crate source.
Other sample programs expecting the same server setup can be found in the `examples` subdirectory.

### Synchronous search

```rust
use ldap3::{LdapConn, Scope, SearchEntry};
use ldap3::result::Result;

fn main() -> Result<()> {
    let mut ldap = LdapConn::new("ldap://localhost:2389")?;
    let (rs, _res) = ldap.search(
        "ou=Places,dc=example,dc=org",
        Scope::Subtree,
        "(&(objectClass=locality)(l=ma*))",
        vec!["l"]
    )?.success()?;
    for entry in rs {
        println!("{:?}", SearchEntry::construct(entry));
    }
    Ok(ldap.unbind()?)
}
```

### Asynchronous search

```rust
use ldap3::{LdapConnAsync, Scope, SearchEntry};
use ldap3::result::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let (conn, mut ldap) = LdapConnAsync::new("ldap://localhost:2389").await?;
    ldap3::drive!(conn);
    let (rs, _res) = ldap.search(
        "ou=Places,dc=example,dc=org",
        Scope::Subtree,
        "(&(objectClass=locality)(l=ma*))",
        vec!["l"]
    ).await?.success()?;
    for entry in rs {
        println!("{:?}", SearchEntry::construct(entry));
    }
    Ok(ldap.unbind().await?)
}
```

## Compile-time features

The following features are available at compile time:

* __sync__ (enabled by default): Synchronous API support.

* __tls__ (enabled by default): TLS support, backed by the `native-tls` crate, which uses
 a platform-specific TLS backend. This is an alias for __tls-native__.

* __tls-rustls__ (disabled by default): TLS support, backed by the Rustls library.

Without any features, only plain TCP connections (and Unix domain sockets on Unix-like
platforms) are available. For TLS support, __tls__ and __tls-rustls__ are mutually
exclusive: choosing both will produce a compile-time error.

## License

Licensed under either of:

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)), or
 * MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
