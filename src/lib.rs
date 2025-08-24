#![deny(clippy::all)]
#![feature(trait_alias)]

pub mod manifest;
pub mod ssl;
pub mod version;
pub mod script;

#[macro_use]
extern crate napi_derive;
