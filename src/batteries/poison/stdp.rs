use crate::api::PoisonPolicy;
use core::sync::atomic::{AtomicBool, Ordering};

/// A poison policy that uses `std::thread::panicking()` to detect panics.
///
/// This is the default policy when the `std` feature is enabled. It stores
/// the poisoned state in an [`AtomicBool`].
#[cfg(any(std, docsrs))]
#[derive(Debug, Default)]
pub struct StdPoison(AtomicBool);

#[cfg(any(std, docsrs))]
impl PoisonPolicy for StdPoison
{
    #[inline]
    fn is_poisoned(&self) -> bool
    {
        self.0.load(Ordering::Acquire)
    }

    #[inline]
    fn on_drop(&self)
    {
        if std::thread::panicking()
        {
            self.0.store(true, Ordering::Release);
        }
    }

    #[inline]
    unsafe fn clear_poison(&self)
    {
        self.0.store(false, Ordering::Release);
    }
}
