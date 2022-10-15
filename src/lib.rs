#![warn(missing_docs)]

//! Relay style pagination for Juniper.
//!
//! To return a [connection][spec] from your resolver, just call
//! [`RelayConnection::new`](RelayConnection::new) providing a closure
//! to load the connection nodes.
//!
//! # Example
//!
//! ```
//! # use juniper::GraphQLObject;
//! # use juniper_relay::{RelayConnection, RelayConnectionNode};
//! #
//! #[derive(GraphQLObject)]
//! struct Foo {
//!   id: i32,
//! }
//!
//! impl RelayConnectionNode for Foo {
//!     type Cursor = i32;
//!     fn cursor(&self) -> Self::Cursor {
//!         self.id
//!     }
//!     fn connection_type_name() -> &'static str {
//!         "FooConnection"
//!     }
//!     fn edge_type_name() -> &'static str {
//!         "FooConnectionEdge"
//!     }
//! }
//!
//! # let first = Some(42);
//! # let after = Some("42".into());
//! # let last = None;
//! # let before = None;
//! # fn run_query(sql: String) -> Vec<Foo> { vec![] };
//! #
//! RelayConnection::new(first, after, last, before, |after, before, limit| {
//!     let sql = format!(
//!         "SELECT (id) FROM foo WHERE id > {after:?} AND id < {before:?} LIMIT {limit:?}"
//!     );
//!     let edges: Vec<Foo> = run_query(sql);
//!     Ok(edges)
//! })
//! # ;
//! ```
//!
//! [spec]: https://relay.dev/graphql/connections.htm

use juniper::{FieldResult, GraphQLObject};
use std::convert::TryInto;

mod traits;

/// To return objects inside a connection, they must
/// implement this trait.
pub trait RelayConnectionNode {
    /// The [cursor][spec] type that is used for pagination. A cursor
    /// should uniquely identify a given node.
    ///
    /// [spec]: https://relay.dev/graphql/connections.htm#sec-Cursor
    type Cursor: std::string::ToString + std::str::FromStr + Clone;

    /// Returns the cursor associated with this node.
    fn cursor(&self) -> Self::Cursor;

    /// Returns the type name connections
    /// over these nodes should have in the
    /// API. E.g. `"FooConnection"`.
    fn connection_type_name() -> &'static str;

    /// Returns the type name edges containing
    /// these nodes should have in the API.
    /// E.g. `"FooConnectionEdge"`.
    fn edge_type_name() -> &'static str;
}

#[derive(Debug)]
#[doc(hidden)]
pub struct RelayConnectionEdge<N> {
    node: N,
    cursor: String,
}

#[derive(Debug, GraphQLObject)]
#[graphql(name = "PageInfo")]
#[doc(hidden)]
pub struct RelayConnectionPageInfo {
    has_previous_page: bool,
    has_next_page: bool,
    start_cursor: Option<String>,
    end_cursor: Option<String>,
}

/// Implements the relay connection [specification][spec], and allows to
/// easily paginate over any given list of GraphQL objects.
///
/// [spec]: https://relay.dev/graphql/connections.htm
#[derive(Debug)]
pub struct RelayConnection<N> {
    edges: Vec<RelayConnectionEdge<N>>,
    page_info: RelayConnectionPageInfo,
}

fn leq_zero(val: i64) -> Result<i64, &'static str> {
    if val < 0 {
        Err("Pagination argument must be positive")
    } else {
        Ok(val)
    }
}

impl<N> RelayConnection<N>
where
    N: RelayConnectionNode,
    <N::Cursor as std::str::FromStr>::Err: std::fmt::Display,
{
    fn closure_args(
        first: Option<i64>,
        after: Option<String>,
        before: Option<String>,
    ) -> FieldResult<(Option<N::Cursor>, Option<N::Cursor>, Option<i64>)> {
        let after: Option<N::Cursor> = after.map(|s| s.parse()).transpose()?;
        let before: Option<N::Cursor> = before.map(|s| s.parse()).transpose()?;

        // to ensure `hasNextPage` can be set correctly
        let limit = first.map(|l| l + 1);

        Ok((after, before, limit))
    }

    fn build_connection(
        first: Option<i64>,
        last: Option<i64>,
        edges: Vec<N>,
    ) -> FieldResult<RelayConnection<N>> {
        let edges_len: i64 = edges.len().try_into()?;

        let has_previous_page = if let Some(last) = last {
            edges_len > last
        } else {
            false
        };
        let has_next_page = if let Some(first) = first {
            edges_len > first
        } else {
            false
        };

        let first = first.unwrap_or(edges_len);
        let last = last.unwrap_or(edges_len);

        let len_after_take = i64::min(edges_len, first);
        let skip = i64::max(0, len_after_take - last);

        let edges: Vec<RelayConnectionEdge<N>> = edges
            .into_iter()
            .take(first.try_into()?)
            .skip(skip.try_into()?)
            .map(|node| RelayConnectionEdge {
                cursor: node.cursor().to_string(),
                node,
            })
            .collect();

        Ok(RelayConnection {
            page_info: RelayConnectionPageInfo {
                has_previous_page,
                has_next_page,
                start_cursor: edges.first().map(|edge| edge.cursor.clone()),
                end_cursor: edges.last().map(|edge| edge.cursor.clone()),
            },
            edges,
        })
    }

    /// Build a relay-style paginated list. You must supply a
    /// closure which is used to load the data from some backing
    /// store. It takes arguments: `after: Option<C>`,
    /// `before: Option<C>`, and `limit: Option<i64>`.
    ///
    /// The `limit` argument is purely an optimization and may
    /// be ignored without breaking the connection specification.
    ///
    /// The arguments correspond to SQL in the following way:
    /// ```SQL
    /// SELECT ... FROM table WHERE cursor > $after AND cursor < $before LIMIT $limit
    /// ```
    pub fn new<L>(
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
        load: L,
    ) -> FieldResult<RelayConnection<N>>
    where
        L: FnOnce(Option<N::Cursor>, Option<N::Cursor>, Option<i64>) -> FieldResult<Vec<N>>,
    {
        let first: Option<i64> = first.map(Into::into).map(leq_zero).transpose()?;
        let last: Option<i64> = last.map(Into::into).map(leq_zero).transpose()?;
        let (after, before, limit) = Self::closure_args(first, after, before)?;
        let edges = load(after, before, limit)?;
        Self::build_connection(first, last, edges)
    }

    /// The same as [`new`](Self::new), but with an `async` closure.
    pub async fn new_async<L, F>(
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
        load: L,
    ) -> FieldResult<RelayConnection<N>>
    where
        L: FnOnce(Option<N::Cursor>, Option<N::Cursor>, Option<i64>) -> F,
        F: std::future::Future<Output = FieldResult<Vec<N>>>,
    {
        let first: Option<i64> = first.map(Into::into).map(leq_zero).transpose()?;
        let last: Option<i64> = last.map(Into::into).map(leq_zero).transpose()?;
        let (after, before, limit) = Self::closure_args(first, after, before)?;
        let edges = load(after, before, limit).await?;
        Self::build_connection(first, last, edges)
    }

    /// Returns a relay connection with no elements.
    pub fn empty() -> Self {
        Self {
            edges: vec![],
            page_info: RelayConnectionPageInfo {
                has_previous_page: false,
                has_next_page: false,
                start_cursor: None,
                end_cursor: None,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(GraphQLObject)]
    struct FakeNode {
        id: i32,
    }

    impl RelayConnectionNode for FakeNode {
        type Cursor = i32;

        fn cursor(&self) -> Self::Cursor {
            self.id
        }

        fn connection_type_name() -> &'static str {
            "FakeNodeConnection"
        }

        fn edge_type_name() -> &'static str {
            "FakeNodeConnectionEdge"
        }
    }

    #[test]
    fn closure_args_smoke_test() {
        assert_eq!(
            RelayConnection::<FakeNode>::closure_args(Some(42), Some("8".into()), None),
            Ok((Some(8), None, Some(43)))
        );
        assert_eq!(
            RelayConnection::<FakeNode>::closure_args(None, None, Some("95".into())),
            Ok((None, Some(95), None))
        );
        assert!(
            RelayConnection::<FakeNode>::closure_args(None, Some("foo".to_string()), None).is_err()
        );
    }
}
