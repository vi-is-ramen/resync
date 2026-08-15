//! A shared RAII guard that provides shared (read) access to the protected
//! data.
//!
//! When this guard is dropped, the shared lock is automatically released
//! via the [`SharingPolicy::free_share`] method.

use crate::traits::{LockPolicy, SharingPolicy};
use core::ops::Deref;

/// A shared RAII guard that provides shared (read) access to the protected
/// data.
///
/// When this guard is dropped, the shared lock is automatically released
/// via the [`SharingPolicy::free_share`] method.
#[allow(missing_debug_implementations)]
pub struct ShGuard<'a, T, L, M = <L as LockPolicy>::Meta>
where L: SharingPolicy<Meta = M>
{
    data: *const T,
    lock: &'a L,
    meta: M,
}

unsafe impl<'a, T, L, M> core::marker::Send for ShGuard<'a, T, L, M>
where
    T: Send,
    L: SharingPolicy<Meta = M> + Send,
    M: Send,
{
}

impl<'a, T, L, M> ShGuard<'a, T, L, M>
where L: SharingPolicy<Meta = M>
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
    pub fn new(data: *const T, lock: &'a L, meta: M) -> Self
    {
        Self { data, lock, meta }
    }
}

impl<'a, T, L, M> core::fmt::Debug for ShGuard<'a, T, L, M>
where
    T: core::fmt::Debug,
    L: SharingPolicy<Meta = M>,
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

impl<'a, T, L, M> core::ops::Drop for ShGuard<'a, T, L, M>
where L: SharingPolicy<Meta = M>
{
    fn drop(&mut self)
    {
        self.lock.free_share(&self.meta);
    }
}

impl<'a, T, L, M> Deref for ShGuard<'a, T, L, M>
where L: SharingPolicy<Meta = M>
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
