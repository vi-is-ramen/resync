//! A shared RAII guard that provides shared (read) access to the protected
//! data.
//!
//! When this guard is dropped, the shared lock is automatically released
//! via the [`SharingPolicy::free_share`] method.

use core::ops::Deref;

use crate::traits::SharingPolicy;

/// A shared RAII guard that provides shared (read) access to the protected
/// data.
///
/// When this guard is dropped, the shared lock is automatically released
/// via the [`SharingPolicy::free_share`] method.
#[allow(missing_debug_implementations)]
pub struct ShGuard<'a, T, L>
where L: SharingPolicy
{
    data: *const T,
    lock: &'a L,
}

impl<'a, T, L> ShGuard<'a, T, L>
where L: SharingPolicy
{
    /// Creates a new shared guard.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `data` points to valid, initialized data protected by the lock.
    /// - The shared (reader) lock has been successfully acquired on `lock`.
    /// - No mutable references to the protected data exist while this guard is
    ///   alive.
    pub fn new(data: *const T, lock: &'a L) -> Self
    {
        Self { data, lock }
    }
}

impl<'a, T, L> core::fmt::Debug for ShGuard<'a, T, L>
where
    T: core::fmt::Debug,
    L: SharingPolicy,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        // SAFETY:
        // The guard guarantees shared access to the data, and no mutable
        // references can exist while this guard is alive.
        let inner = unsafe { self.data.as_ref_unchecked() };
        <T as core::fmt::Debug>::fmt(inner, f)
    }
}

impl<'a, T, L> core::ops::Drop for ShGuard<'a, T, L>
where L: SharingPolicy
{
    fn drop(&mut self)
    {
        self.lock.free_share();
    }
}

impl<'a, T, L> Deref for ShGuard<'a, T, L>
where L: SharingPolicy
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        // SAFETY:
        // The guard guarantees shared access to the data, and no mutable
        // references can exist while this guard is alive.
        unsafe { self.data.as_ref_unchecked() }
    }
}
