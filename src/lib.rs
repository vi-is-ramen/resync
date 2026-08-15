//! A LEGO-like synchronization primitives library.
//!
//! This crate provides composable building blocks for implementing locks and
//! spin loops. Instead of a one-size-fits-all mutex, Resync allows you to
//! mix and match lock acquisition strategies and retry backends at compile
//! time using generic traits.
//!
//! # Core Concepts
//!
//! - **[`traits::LockPolicy`]**: Defines how to atomically acquire and release
//!   an exclusive lock.
//! - **[`traits::SharingPolicy`]**: Extends `LockPolicy` to support shared
//!   (reader) access.
//! - **[`traits::RetryPolicy`]**: Defines what to do while waiting for a
//!   contended lock (e.g., spin or yield).
//! - **[`Mutex`]**: A high-level primitive that composes a lock policy and a
//!   retry policy to protect data.
//!
//! # Features
//!
//! - **`std`** *(enabled by default)*: Enables OS-based retry
//!   ([`retry::Yield`]) and OS-specific lock ([`lock::Os`]) backends using
//!   futexes (Linux), `pthread_rwlock_t` (macOS), or `SRWLOCK` (Windows).
//! - **`no_std`**: If the `std` feature is disabled, the crate becomes
//!   `#![no_std]` compatible. The default retry strategy falls back to
//!   [`retry::Busy`], which issues `core::hint::spin_loop()`, and the lock
//!   backend falls back to [`lock::Atomic`].
//!
//! # Guidebook
//!
//! For a comprehensive guide on the library's philosophy, design decisions,
//! and advanced usage patterns, see the [`guide`] module.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg(feature = "std")]
extern crate libc;

mod batteries;
pub mod traits;
mod util;

pub use batteries::primitives::*;
pub use batteries::*;

mod result;
pub use result::*;

/// The comprehensive guidebook for the `resync` crate.
///
/// This module contains the full text of the Resync Guidebook, covering
/// the library's philosophy, core concepts, advanced usage patterns, design
/// decisions, and inherent limitations.
#[doc = include_str!("../markdown/book.md")]
pub mod guide
{}
