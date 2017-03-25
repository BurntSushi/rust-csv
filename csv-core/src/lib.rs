/*!
Docs.
*/

#![allow(dead_code, unused_imports, unused_mut, unused_variables)]

// #![cfg_attr(not(test), no_std)]

#[cfg(test)]
extern crate arrayvec;
extern crate core;

pub use reader::{Reader, ReaderBuilder, ReadResult, Terminator};

mod reader;
