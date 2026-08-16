//! Generic traits and abstractions for synchronization primitives.
//!
//! This module provides behavior-driven traits that abstract over the
//! concrete implementations of locks in `resync`. While concrete structs like
//! [`Mutex`](crate::Mutex) and [`Sharex`](crate::Sharex) are heavily
//! parameterized by their [`LockPolicy`](crate::traits::LockPolicy) and
//! [`RetryPolicy`](crate::traits::RetryPolicy), the traits in this module
//! allow you to write generic code that accepts *any* compatible
//! synchronization primitive.
//!
//! # Philosophy
//!
//! This module serves as `resync`'s answer to the `lock_api` crate. Because
//! `resync` embraces granular error handling (via
//! [`AcquireError`](crate::AcquireError) and
//! [`TryLockError`](crate::TryLockError)) and lock poisoning, it cannot
//! directly implement `lock_api`'s infallible `RawMutex` trait. Instead,
//! `resync::api` provides a richer, more expressive set of traits that preserve
//! these safety guarantees while enabling ecosystem interoperability.

pub(crate) mod mutex;
pub(crate) mod sharex;

// Re-export core traits so users can access them via `resync::api::*`
pub use crate::traits::*;
pub use mutex::*;
pub use sharex::*;
