//! A busy‑wait spin strategy that uses [`core::hint::spin_loop`].

use crate::{ISpin, SpinResult};

/// A spin strategy that executes a CPU pause instruction (or its equivalent).
///
/// This is suitable for short‑term spinning where yielding to the OS is
/// unnecessary and might be too costly.
///
/// # Examples
/// ```
/// # use resync::ISpin;
/// use resync::SpinResult;
/// use resync::spin::Busy;
///
/// let spin = Busy;
/// assert_eq!(spin.spin(), SpinResult::Ok);
/// ```
#[allow(missing_debug_implementations)]
pub struct Busy;

#[cfg(nightly)]
const impl core::default::Default for Busy
{
    fn default() -> Self
    {
        Self
    }
}

#[cfg(not(nightly))]
impl core::default::Default for Busy
{
    fn default() -> Self
    {
        Self
    }
}

impl ISpin for Busy
{
    /// Issues a [`core::hint::spin_loop`] hint and returns [`SpinResult::Ok`].
    ///
    /// # Returns
    /// Always [`SpinResult::Ok`].
    fn spin(&self) -> SpinResult
    {
        core::hint::spin_loop();
        SpinResult::Ok
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
        assert_eq!(spin.spin(), SpinResult::Ok);
    }

    #[test]
    fn busy_default_works()
    {
        let spin = Busy::default();
        assert_eq!(spin.spin(), SpinResult::Ok);
    }
}
