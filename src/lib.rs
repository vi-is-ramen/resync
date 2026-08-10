//! A LEGO-like synchronization primitives library.
//!
//! This crate provides building blocks for implementing locks and spin loops,
//! with composable traits and backends that can be swapped at compile time.
//!
//! Guidebook: [`guide`]
//!
//! # Features
//! - `std` (enabled by default): enables OS‑based spinning ([`spin::Os`]).

// don't link to libstd if `std` feature disabled
#![cfg_attr(not(feature = "std"), no_std)]

pub mod lock;
mod prim;
mod result;
pub mod spin;

pub use lock::ILock;
pub use prim::*;
pub use result::*;
pub use spin::ISpin;

pub mod guide;
