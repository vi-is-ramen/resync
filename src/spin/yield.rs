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
/// use resync::spin::Yield;
///
/// let spin = Yield;
/// assert_eq!(spin.spin(), SpinResult::Ok);
/// ```
#[derive(Default, Debug)]
pub struct Yield;

impl Yield
{
    /// Creates new instance of [`Os`] spin strategy.
    pub const fn new() -> Self
    {
        Self
    }
}

impl ISpin for Yield
{
    /// Yields the current thread and returns [`SpinResult::Ok`].
    ///
    /// # Returns
    /// Always [`SpinResult::Ok`].
    fn spin(&self) -> SpinResult
    {
        core::hint::spin_loop();
        SpinResult::Ok
    }
}
