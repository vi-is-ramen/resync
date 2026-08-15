//! A cooperative retry strategy that yields the current thread to the OS
//! scheduler.
//!
//! This module provides the [`Yield`] retry policy, which is the default
//! waiting strategy when the `std` feature is enabled. Instead of burning
//! CPU cycles in a tight loop, it calls [`std::thread::yield_now`] to give
//! other threads (including the one holding the lock) a chance to run.

use core::convert::Infallible;

use crate::RetryResult;
use crate::traits::RetryPolicy;

/// A spin strategy that yields the current thread to the OS scheduler on every
/// retry.
///
/// This policy is highly cooperative and prevents CPU starvation in user-space
/// applications. However, it may introduce higher latency compared to a pure
/// busy-wait if the lock is released very quickly.
#[derive(Default, Debug)]
pub struct Yield;

impl Yield
{
    /// Creates a new instance of the [`Yield`] retry strategy.
    ///
    /// This is a `const` function, allowing the strategy to be initialized
    /// in static variables.
    pub const fn new() -> Self
    {
        Self
    }
}

impl RetryPolicy for Yield
{
    type Error = Infallible;

    /// Performs one yield iteration by invoking [`std::thread::yield_now`].
    ///
    /// This method never aborts, so it always returns `Ok(())`.
    fn retry(&self, _: usize) -> RetryResult<Self::Error>
    {
        std::thread::yield_now();
        Ok(())
    }
}
