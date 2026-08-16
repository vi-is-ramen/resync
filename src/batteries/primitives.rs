//! High-level synchronization primitives built on top of lock and retry
//! policies.
//!
//! This module contains the user-facing synchronization primitives, such as
//! [`Mutex`], [`Sharex`], [`Gate`], [`Semaphore`], and [`Condvar`]. These
//! primitives combine a [`LockPolicy`](crate::traits::LockPolicy) or
//! [`SharingPolicy`](crate::traits::SharingPolicy) and a
//! [`RetryPolicy`](crate::traits::RetryPolicy) into a safe, RAII-based API.
//!
//! # Lock Poisoning
//!
//! When compiled with the `std` feature, primitives that protect user data
//! ([`Mutex`] and [`Sharex`]) automatically detect if a thread panics while
//! holding the lock. If a panic occurs, the lock is marked as **poisoned**.
//! Subsequent attempts to acquire the lock will return an
//! [`AcquireError::Poisoned`](crate::AcquireError::Poisoned) or
//! [`TryLockError::Poisoned`](crate::TryLockError::Poisoned), allowing the
//! caller to inspect and repair the potentially inconsistent data.

#[cfg(dev)]
pub(crate) mod barrier;
#[cfg(dev)]
pub(crate) mod condvar;
pub(crate) mod exguard;
#[cfg(dev)]
pub(crate) mod gate;
pub(crate) mod mutex;
#[cfg(dev)]
pub(crate) mod sem;
pub(crate) mod sharex;
pub(crate) mod shguard;

#[cfg(dev)]
pub use barrier::*;
#[cfg(dev)]
pub use condvar::*;
pub use exguard::*;
#[cfg(dev)]
pub use gate::*;
pub use mutex::*;
#[cfg(dev)]
pub use sem::*;
pub use sharex::*;
pub use shguard::*;
