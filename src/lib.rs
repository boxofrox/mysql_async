// Copyright (c) 2016 Anatoly Ikorsky
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! ## mysql-async
//! Tokio based asynchronous MySql client library for rust programming language.
//!
//! ### Installation
//! Library hosted on [crates.io](https://crates.io/crates/mysql_async/).
//!
//! ```toml
//! [dependencies]
//! mysql = "<desired version>"
//! ```
//!
//! ### Example
//!
//! ```rust
//! extern crate futures;
//! #[macro_use]
//! extern crate mysql_async as my;
//! extern crate tokio_core as tokio;
//! // ...
//!
//! use futures::Future;
//! use my::prelude::*;
//! use tokio::reactor::Core;
//! # use std::env;
//!
//! #[derive(Debug, PartialEq, Eq)]
//! struct Payment {
//!     customer_id: i32,
//!     amount: i32,
//!     account_name: Option<String>,
//! }
//!
//! fn main() {
//!     let mut lp = Core::new().unwrap();
//!
//!     let payments = vec![
//!         Payment { customer_id: 1, amount: 2, account_name: None },
//!         Payment { customer_id: 3, amount: 4, account_name: Some("foo".into()) },
//!         Payment { customer_id: 5, amount: 6, account_name: None },
//!         Payment { customer_id: 7, amount: 8, account_name: None },
//!         Payment { customer_id: 9, amount: 10, account_name: Some("bar".into()) },
//!     ];
//!
//!     # let database_url: String = if let Ok(url) = env::var("DATABASE_URL") {
//!     #     let opts = my::Opts::from_url(&url).expect("DATABASE_URL invalid");
//!     #     if opts.get_db_name().expect("a database name is required").is_empty() {
//!     #         panic!("database name is empty");
//!     #     }
//!     #     url
//!     # } else {
//!     #     "mysql://root:password@127.0.0.1:3307/mysql".into()
//!     # };
//!
//!     let pool = my::Pool::new(database_url, &lp.handle());
//!     let future = pool.get_conn().and_then(|conn| {
//!         // Create temporary table
//!         conn.drop_query(
//!             r"CREATE TEMPORARY TABLE payment (
//!                 customer_id int not null,
//!                 amount int not null,
//!                 account_name text
//!             )"
//!         )
//!     }).and_then(|conn| {
//!         // Save payments
//!         let params = payments.iter().map(|payment| {
//!             params! {
//!                 "customer_id" => payment.customer_id,
//!                 "amount" => payment.amount,
//!                 "account_name" => payment.account_name.clone(),
//!             }
//!         }).collect();
//!
//!         conn.batch_exec(r"INSERT INTO payment (customer_id, amount, account_name)
//!                         VALUES (:customer_id, :amount, :account_name)", params)
//!     }).and_then(|conn| {
//!         // Load payments from database.
//!         conn.prep_exec("SELECT customer_id, amount, account_name FROM payment", ())
//!     }).and_then(|result| {
//!         // Collect payments
//!         result.map_and_drop(|row| {
//!             let (customer_id, amount, account_name) = my::from_row(row);
//!             Payment {
//!                 customer_id: customer_id,
//!                 amount: amount,
//!                 account_name: account_name,
//!             }
//!         })
//!     }).and_then(|(_ /* conn */, payments)| {
//!         // The destructor of a connection will return it to the pool,
//!         // but pool should be disconnected explicitly because it's
//!         // an asynchronous procedure.
//!         pool.disconnect().map(|_| payments)
//!     });
//!
//!     let loaded_payments = lp.run(future).unwrap();
//!
//!     assert_eq!(loaded_payments, payments);
//! }
//! ```

#![recursion_limit = "1024"]
#![cfg_attr(feature = "nightly", feature(test, const_fn, drop_types_in_const))]

#[cfg(feature = "nightly")]
extern crate test;

#[macro_use]
extern crate bitflags;
extern crate byteorder;
pub extern crate chrono;
#[macro_use]
extern crate error_chain;
extern crate fnv;
#[macro_use]
extern crate futures as lib_futures;
#[macro_use]
extern crate lazy_static;
extern crate mio;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate sha1;
pub extern crate time;
extern crate tokio_core as tokio;
extern crate tokio_io;
extern crate twox_hash;
extern crate url;

#[macro_use]
pub mod macros;
#[macro_use]
mod value;
mod conn;
mod connection_like;
/// Mysql constants
pub mod consts;
/// Errors used in this crate
pub mod errors;
mod io;
mod local_infile_handler;
mod opts;
mod proto;
mod queryable;
mod scramble;

pub type BoxFuture<T> = Box<lib_futures::Future<Item = T, Error = errors::Error>>;

#[doc(inline)]
pub use self::conn::Conn;
#[doc(inline)]
pub use self::conn::pool::Pool;

#[doc(inline)]
pub use self::queryable::transaction::IsolationLevel;

#[doc(inline)]
pub use self::opts::{Opts, OptsBuilder};

#[doc(inline)]
pub use self::local_infile_handler::builtin::WhiteListFsLocalInfileHandler;

#[doc(inline)]
pub use self::proto::{Column, ErrPacket, Row};

#[doc(inline)]
pub use self::value::{from_row, from_row_opt, from_value, from_value_opt, Params, Value,
                      Serialized, Deserialized};

#[doc(inline)]
pub use self::queryable::query_result::QueryResult;

#[doc(inline)]
pub use self::queryable::transaction::{Transaction, TransactionOptions};

/// Futures used in this crate
mod futures {
    pub use queryable::query_result::{
        ForEachAndDrop,
        MapAndDrop,
        ReduceAndDrop,
        ForEach,
        Map,
        Reduce,
    };
}

/// Traits used in this crate
pub mod prelude {
    #[doc(inline)]
    pub use queryable::Queryable;
    #[doc(inline)]
    pub use local_infile_handler::LocalInfileHandler;
    #[doc(inline)]
    pub use value::ConvIr;
    #[doc(inline)]
    pub use value::FromRow;
    #[doc(inline)]
    pub use value::FromValue;
    #[doc(inline)]
    pub use value::ToValue;
}

#[cfg(test)]
mod test_misc {
    use std::env;
    use opts;
    lazy_static! {
        pub static ref DATABASE_URL: String = {
            if let Ok(url) = env::var("DATABASE_URL") {
                let opts = opts::Opts::from_url(&url).expect("DATABASE_URL invalid");
                if opts.get_db_name().expect("a database name is required").is_empty() {
                    panic!("database name is empty");
                }
                url
            } else {
                "mysql://root:password@127.0.0.1:3307/mysql".into()
            }
        };
    }
}
