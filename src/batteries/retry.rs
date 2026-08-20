//! Built-in implementations of [`RetryPolicy`](crate::traits::RetryPolicy).
//!
//! This module provides ready-to-use retry strategies that can be plugged
//! into higher-level primitives like [`Mutex`](crate::Mutex). The retry policy
//! determines what the CPU should do when a lock acquisition attempt fails
//! due to contention.

mod busy;
#[cfg(any(std, docsrs))]
mod r#yield;

#[cfg(any(docsrs, feature = "fake"))]
pub use super::fake::*;
pub use busy::*;
#[cfg(any(std, docsrs))]
pub use r#yield::*;

/// Default retry policy for current environment.
///
/// As `std` feature enabled, it is `Yield`.
#[cfg(all(std, not(docsrs)))]
pub type DefaultRetry = Yield;

/// Default retry policy for current environment.
///
/// As `std` feature disabled, it is `Busy`.
#[cfg(all(no_std, not(docsrs)))]
pub type DefaultRetry = Busy;

/// Default retry policy for current environment.
#[cfg(docsrs)]
pub type DefaultRetry = Fake;
