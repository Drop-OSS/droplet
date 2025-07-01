#![deny(clippy::all)]
#![feature(trait_alias)]

pub mod manifest;
pub mod ssl;
pub mod version;

#[macro_use]
extern crate napi_derive;
