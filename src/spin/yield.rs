use core::convert::Infallible;

use crate::{ISpin, SpinResult};

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

impl ISpin for Yield
{
    type Error = Infallible;

    #[cfg(feature = "std")]
    fn spin(&self) -> SpinResult<Self::Error>
    {
        std::thread::yield_now();
        Ok(())
    }

    #[cfg(not(feature = "std"))]
    fn spin(&self) -> SpinResult<Self::Error>
    {
        core::hint::spin_loop();
        Ok(())
    }
}
