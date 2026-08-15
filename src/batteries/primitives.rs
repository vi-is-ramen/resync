//! High-level synchronization primitives built on top of lock and retry
//! policies.
//!
//! This module contains the user-facing synchronization primitives, such as
//! [`Mutex`], [`Sharex`], and [`Barrier`], which combine a
//! [`LockPolicy`](crate::traits::LockPolicy) or
//! [`SharingPolicy`](crate::traits::SharingPolicy) and a
//! [`RetryPolicy`](crate::traits::RetryPolicy) into a safe, RAII-based API.

#[cfg(dev)]
mod barrier;
mod exguard;
mod mutex;
#[cfg(dev)]
mod newsitem;
mod sharex;
mod shguard;

#[cfg(dev)]
pub use barrier::*;
pub use exguard::*;
pub use mutex::*;
#[cfg(dev)]
pub use newsitem::*;
pub use sharex::*;
pub use shguard::*;
