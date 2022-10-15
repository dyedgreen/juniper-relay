# Juniper Relay Connections

[![crates.io](https://img.shields.io/crates/v/placeholder.svg)](https://crates.io/crates/placeholder)
[![Released API docs](https://docs.rs/placeholder/badge.svg)](https://docs.rs/placeholder)
[![CI](https://github.com/dyedgreen/juniper-relay/actions/workflows/ci.yml/badge.svg)](https://github.com/dyedgreen/juniper-relay/actions/workflows/ci.yml)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Relay style pagination for Juniper.

This library provides the a `RelayConnection` struct, which can be returned in a Juniper GraphQL
schema and implements the [relay connection interface][spec].

## Example

```rust
use juniper_relay::{RelayConnection, RelayConnectionNode};

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

let first: Option<i32> = Some(42);
let after: Option<String> = Some("42");
let last: Option<i32> = None;
let before: Option<String> = None;

fn run_query(sql: String) -> Vec<Foo> { vec![] };

RelayConnection::new(first, after, last, before, |after, before, limit| {
    // You'd typically want use a query builder like diesel for this ...
    let sql = format!("SELECT (id) FROM foo WHERE id > {after} AND id < {before} LIMIT {limit}");
    Ok(run_query(sql))
})
```

[spec]: https://relay.dev/graphql/connections.htm
