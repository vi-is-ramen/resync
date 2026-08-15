//! High-level synchronization primitives built on top of lock and retry
//! policies.
//!
//! This module contains the user-facing synchronization primitives, such as
//! [`Mutex`] and [`Sharex`], which combine a
//! [`LockPolicy`](crate::traits::LockPolicy) or
//! [`SharingPolicy`](crate::traits::SharingPolicy) and a
//! [`RetryPolicy`](crate::traits::RetryPolicy) into a safe, RAII-based API.

mod mutex;
mod sharex;

pub use mutex::*;
pub use sharex::*;
