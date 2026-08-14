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

pub mod lock;
mod prim;
mod result;
pub mod share;
pub mod spin;

pub use lock::{DEFAULT_EPSILON, ILock};
pub use prim::{Barrier, Gate, Mutex, MutexGuard, RwLock, RwMut, RwRef};
pub use result::*;
pub use share::IShare;
pub use spin::ISpin;

#[doc = include_str!("../markdown/book.md")]
pub mod guide
{}
