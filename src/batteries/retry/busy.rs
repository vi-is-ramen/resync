//! A busy‑wait spin strategy that uses [`core::hint::spin_loop`].

use core::convert::Infallible;

use crate::RetryResult;
use crate::traits::RetryPolicy;

/// A spin strategy that executes a CPU pause instruction.
#[allow(missing_debug_implementations)]
pub struct Busy;

impl core::default::Default for Busy
{
    fn default() -> Self
    {
        Self
    }
}

impl RetryPolicy for Busy
{
    type Error = Infallible;

    fn retry(&self, _: usize) -> RetryResult<Self::Error>
    {
        core::hint::spin_loop();
        Ok(())
    }
}
