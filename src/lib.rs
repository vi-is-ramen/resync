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
//! - **[`traits::NewLocked`]**: Allows locks to be initialized in an already
//!   acquired state, preventing TOCTOU races in primitives like [`Gate`].
//! - **[`traits::PoisonPolicy`]**: Defines how a lock reacts to thread panics.
//!   Use [`poison::NoPoison`] for zero-overhead critical sections, or implement
//!   your own for custom `no_std` unwinding environments.
//! - **[`Mutex`]** / **[`Sharex`]**: High-level primitives that compose
//!   policies to protect data.
//! - **[`Gate`]**: A controllable barrier that blocks thread flow until opened.
//! - **[`Semaphore`]**: A counting semaphore for resource pooling.
//! - **[`Condvar`]**: A condition variable for event-based waiting.
//!
//! # Features
//!
//! - **`std`** *(enabled by default)*: Enables OS-based retry
//!   ([`retry::Yield`]), OS-specific lock ([`lock::Os`]) backends using futexes
//!   (Linux), `pthread_rwlock_t` (macOS), or `SRWLOCK` (Windows). Also enables
//!   [`Condvar`] and standard lock poisoning ([`poison::StdPoison`]).
//! - **`no_std`**: If the `std` feature is disabled, the crate becomes
//!   `#![no_std]` compatible. The default retry strategy falls back to
//!   [`retry::Busy`], the lock backend falls back to [`lock::Atomic`], and the
//!   default poison policy falls back to [`poison::NoPoison`].
//!
//! # Guidebook
//!
//! For a comprehensive, interactive guide on the library's philosophy, design
//! decisions, and advanced usage patterns, please visit the **[Resync Book](https://vi-is-ramen.github.io/resync/)**.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg(feature = "std")]
#![allow(type_alias_bounds)]
extern crate libc;

pub(crate) mod batteries;
pub mod traits;
pub(crate) mod util;

pub use batteries::primitives::*;
pub use batteries::*;

mod result;
pub use result::*;

pub mod api;

/// Re-export of the `poison` module for convenient access to poison policies.
pub mod poison
{
    pub use crate::batteries::poison::*;
}
