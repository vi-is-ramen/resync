//! An OS‑based spin strategy that yields the current thread.

use crate::{ISpin, SpinResult};

/// A spin strategy that calls [`std::thread::yield_now`].
///
/// This is appropriate for longer spins or when running on a preemptive
/// scheduler, as it gives other threads a chance to run.
///
/// # Examples
/// ```
/// # use resync::ISpin;
/// use resync::SpinResult;
/// use resync::spin::Os;
///
/// let spin = Os;
/// assert_eq!(spin.spin(), SpinResult::Ok);
/// ```
#[allow(missing_debug_implementations)]
pub struct Os;

#[cfg(nightly)]
const impl core::default::Default for Os
{
    fn default() -> Self
    {
        Self
    }
}

#[cfg(not(nightly))]
impl core::default::Default for Os
{
    fn default() -> Self
    {
        Self
    }
}

impl ISpin for Os
{
    /// Yields the current thread and returns [`SpinResult::Ok`].
    ///
    /// # Returns
    /// Always [`SpinResult::Ok`].
    fn spin(&self) -> SpinResult
    {
        std::thread::yield_now();
        SpinResult::Ok
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn os_spin_returns_ok()
    {
        let spin = Os;
        assert_eq!(spin.spin(), SpinResult::Ok);
    }
}
