//! A busy‑wait spin strategy that uses [`core::hint::spin_loop`].

use core::convert::Infallible;

use crate::{ISpin, SpinResult};

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

impl ISpin for Busy
{
    type Error = Infallible;

    fn spin(&self) -> SpinResult<Self::Error>
    {
        core::hint::spin_loop();
        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn busy_spin_returns_ok()
    {
        let spin = Busy;
        assert_eq!(spin.spin(), Ok(()));
    }
}
