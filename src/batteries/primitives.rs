//! High-level synchronization primitives built on top of lock and retry
//! policies.
//!
//! This module contains the user-facing synchronization primitives, such as
//! [`Mutex`], [`Sharex`], [`Gate`], [`Semaphore`], [`Condvar`], and [`Once`].
//! These primitives combine a [`LockPolicy`](crate::traits::LockPolicy) or
//! [`SharingPolicy`](crate::traits::SharingPolicy), a
//! [`RetryPolicy`](crate::traits::RetryPolicy), and a
//! [`PoisonPolicy`](crate::traits::PoisonPolicy) into a safe, RAII-based API.

#[cfg(dev)]
pub(crate) mod barrier;
#[cfg(dev)]
pub(crate) mod condvar;
pub(crate) mod exguard;
#[cfg(dev)]
pub(crate) mod gate;
pub(crate) mod mutex;
pub(crate) mod once;
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
pub use once::*;
#[cfg(dev)]
pub use sem::*;
pub use sharex::*;
pub use shguard::*;
