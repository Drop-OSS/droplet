#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![feature(trait_alias)]
#![feature(iterator_try_collect)]


pub mod manifest;
pub mod script;
pub mod ssl;
pub mod version;

#[macro_use]
extern crate napi_derive;