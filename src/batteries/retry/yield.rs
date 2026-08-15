use core::convert::Infallible;

use crate::RetryResult;
use crate::traits::RetryPolicy;

/// A spin strategy that calls [`std::thread::yield_now`].
#[derive(Default, Debug)]
pub struct Yield;

impl Yield
{
    /// Creates new instance of [`Yield`] spin strategy.
    pub const fn new() -> Self
    {
        Self
    }
}

impl RetryPolicy for Yield
{
    type Error = Infallible;

    fn retry(&self, _: usize) -> RetryResult<Self::Error>
    {
        std::thread::yield_now();
        Ok(())
    }
}
