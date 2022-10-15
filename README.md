# Juniper Relay Connections

[![crates.io](https://img.shields.io/crates/v/juniper_relay_connection.svg)](https://crates.io/crates/juniper_relay_connection)
[![Released API docs](https://docs.rs/juniper_relay_connection/badge.svg)](https://docs.rs/juniper_relay_connection)
[![CI](https://github.com/dyedgreen/juniper-relay/actions/workflows/ci.yml/badge.svg)](https://github.com/dyedgreen/juniper-relay/actions/workflows/ci.yml)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Relay style pagination for Juniper.

This library provides the a `RelayConnection` struct, which can be returned in a Juniper GraphQL
schema and implements the [relay connection interface][spec].

## Example

```rust
#[derive(GraphQLObject)]
struct Foo {
  id: i32,
}

impl RelayConnectionNode for Foo {
    type Cursor = i32;
    fn cursor(&self) -> Self::Cursor {
        self.id
    }
    fn connection_type_name() -> &'static str {
        "FooConnection"
    }
    fn edge_type_name() -> &'static str {
        "FooConnectionEdge"
    }
}

RelayConnection::new(first, after, last, before, |after, before, limit| {
    let sql = format!("SELECT (id) FROM foo WHERE id > {after} AND id < {before} LIMIT {limit}");
    let edges: Vec<Foo> = run_query(sql);
    Ok(edges)
})
```

[spec]: https://relay.dev/graphql/connections.htm
