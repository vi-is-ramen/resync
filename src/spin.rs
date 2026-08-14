//! Spin strategies for wait loops.

mod busy;
mod os;
mod r#yield;

pub use busy::*;
pub use os::*;
pub use r#yield::*;

/// Default spin strategy for current environment.
#[cfg(feature = "std")]
pub type DefaultSpin = Os;

/// Default spin strategy for current environment.
#[cfg(not(feature = "std"))]
pub type DefaultSpin = Busy;

use crate::SpinResult;

/// A trait for spin strategies used while waiting for a lock.
///
/// # Associated Types
/// - `Error`: the error type for spin aborts (timeout, etc.)
///
/// # Required Method
/// - [`ISpin::spin`]: perform a single spin cycle.
///
/// # Returns
/// - `Ok(())`: continue spinning
/// - `Err(e)`: abort spinning
pub trait ISpin
where Self: core::default::Default
{
    /// The error type for spin aborts.
    ///
    /// Use `core::convert::Infallible` for spins that never abort.
    type Error;

    /// Perform one spin iteration.
    ///
    /// # Returns
    /// - `Ok(())`: continue spinning
    /// - `Err(e)`: abort spinning
    fn spin(&self) -> SpinResult<Self::Error>;
}
