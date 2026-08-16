//! Core building blocks and batteries for the `resync` crate.
//!
//! This module organizes the low-level components that make up the
//! synchronization primitives:
//!
//! - [`lock`]: Implementations of [`LockPolicy`](crate::traits::LockPolicy) and
//!   [`SharingPolicy`](crate::traits::SharingPolicy) (e.g., atomic locks,
//!   OS-specific futexes).
//! - [`retry`]: Implementations of [`RetryPolicy`](crate::traits::RetryPolicy)
//!   (e.g., busy-wait, OS yield).
//! - [`poison`]: Implementations of
//!   [`PoisonPolicy`](crate::traits::PoisonPolicy) (e.g., standard panic
//!   detection, zero-overhead no-poison).
//! - [`primitives`]: High-level, user-facing synchronization primitives like
//!   [`Mutex`](crate::Mutex) that combine locks, retry policies, and poison
//!   policies.

pub mod lock;
pub mod poison;
pub mod primitives;
pub mod retry;
