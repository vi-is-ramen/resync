//! A LEGO-like synchronization primitives library.
//!
//! This crate provides building blocks for implementing locks and spin loops,
//! with composable traits and backends that can be swapped at compile time.
//!
//! Guidebook: [`guide`]
//!
//! # Features
//! - `std` (enabled by default): enables OS‑level locks via futex.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg(feature = "std")]
extern crate libc;

mod batteries;
pub mod traits;

pub use batteries::primitives::*;
pub use batteries::*;

mod result;
pub use result::*;

#[doc = include_str!("../markdown/book.md")]
pub mod guide
{}
