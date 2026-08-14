//! Spin strategies for wait loops.

mod busy;
mod os;
mod r#yield;

pub use busy::*;
pub use os::*;
pub use r#yield::*;

/// Default spin strategy for current environment,
/// selected by Resync. Good option if you just
/// writing something platform-aware without
/// deep-minding about synchronization.
#[cfg(feature = "std")]
pub type DefaultSpin = Os;

/// Default spin strategy for current environment,
/// selected by Resync. Good option if you just
/// writing something platform-aware without
/// deep-minding about synchronization.
#[cfg(not(feature = "std"))]
pub type DefaultSpin = Busy;

use crate::SpinResult;

/// A trait for spin strategies used while waiting for a lock.
///
/// # Required Method
/// - [`ISpin::spin`]: perform a single spin cycle (e.g., yield or CPU pause).
///
/// # Errors
/// [`ISpin::spin`] returns a [`SpinResult`]:
/// - [`SpinResult::Ok`]    – spin completed, continue waiting.
/// - [`SpinResult::Abort`] – abort the waiting loop.
///
/// # Panics
/// Implementations should not panic.
pub trait ISpin
where Self: core::default::Default
{
    /// Perform one spin iteration.
    ///
    /// # Returns
    /// A [`SpinResult`] indicating whether to continue or abort.
    fn spin(&self) -> SpinResult;
}
