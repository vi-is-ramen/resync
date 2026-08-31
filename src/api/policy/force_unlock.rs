/// Allows force unlocking of a lock, regardless of the current state.
pub trait ForceUnlock
{
    /// Force unlocks the lock, regardless of the current state.
    ///
    /// # Safety
    ///
    /// Callers must ensure that force unlocking would not cause unexpected
    /// side effects and that [`Drop::drop`] of the lock guard will not
    /// cause inconsistent state of lock.
    unsafe fn force_unlock(&self);
}
