//! Spin strategies for wait loops.

mod busy;
#[cfg(feature = "std")]
mod os;

pub use busy::*;
#[cfg(feature = "std")]
pub use os::*;

#[cfg(feature = "std")]
/// Default spin strategy for current environment,
/// selected by Resync. It's yielding with `std`
/// feature and busy-waiting without `std` feature.
pub type DefaultSpin = Os;

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
#[cfg(nightly)]
pub trait ISpin
where Self: const core::default::Default
{
    /// Perform one spin iteration.
    ///
    /// # Returns
    /// A [`SpinResult`] indicating whether to continue or abort.
    fn spin(&self) -> SpinResult;
}

#[cfg(not(nightly))]
pub trait ISpin
where Self: const core::default::Default
{
    /// Perform one spin iteration.
    ///
    /// # Returns
    /// A [`SpinResult`] indicating whether to continue or abort.
    fn spin(&self) -> SpinResult;
}
