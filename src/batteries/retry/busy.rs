//! A busy-wait retry strategy that uses CPU pause instructions.
//!
//! This module provides the [`Busy`] retry policy, which is ideal for
//! `#![no_std]` environments or situations where the critical section is
//! expected to be extremely short. It repeatedly executes a CPU pause
//! instruction (via [`core::hint::spin_loop`]) to reduce power consumption
//! and bus contention while waiting for a lock to become available.

use core::convert::Infallible;

use crate::RetryResult;
use crate::traits::RetryPolicy;

/// A spin strategy that executes a CPU pause instruction on every retry.
///
/// This policy never yields the thread to the operating system, making it
/// suitable for bare-metal environments or extremely short waits. However,
/// if used in a user-space application with long contention, it may cause
/// CPU starvation and excessive power usage.
#[derive(Default, Debug)]
pub struct Busy;

impl RetryPolicy for Busy
{
    type Error = Infallible;

    /// Performs one busy-wait iteration by invoking [`core::hint::spin_loop`].
    ///
    /// This method never aborts, so it always returns `Ok(())`.
    fn retry(&self, _: usize) -> RetryResult<Self::Error>
    {
        core::hint::spin_loop();
        Ok(())
    }
}
