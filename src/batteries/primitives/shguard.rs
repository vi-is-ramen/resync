//! A shared RAII guard that provides shared (read) access to the protected
//! data.
use crate::traits::{LockPolicy, SharingPolicy};
use core::ops::Deref;
#[cfg(feature = "std")]
use core::sync::atomic::{AtomicBool, Ordering};

/// A shared RAII guard that provides shared (read) access to the protected
/// data.
#[allow(missing_debug_implementations)]
pub struct ShGuard<'a, T, L, M = <L as LockPolicy>::Meta>
where L: SharingPolicy<Meta = M>
{
    data:        *const T,
    lock:        &'a L,
    meta:        M,
    #[cfg(feature = "std")]
    poison_flag: Option<&'a AtomicBool>,
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
    /// Creates a new shared guard (with `std` feature enabled).
    #[cfg(feature = "std")]
    pub fn new(
        data: *const T,
        lock: &'a L,
        meta: M,
        poison_flag: Option<&'a AtomicBool>,
    ) -> Self
    {
        Self {
            data,
            lock,
            meta,
            poison_flag,
        }
    }

    /// Creates a new shared guard (without `std` feature).
    #[cfg(not(feature = "std"))]
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
        let inner = unsafe { self.data.as_ref_unchecked() };
        <T as core::fmt::Debug>::fmt(inner, f)
    }
}

impl<'a, T, L, M> core::ops::Drop for ShGuard<'a, T, L, M>
where L: SharingPolicy<Meta = M>
{
    fn drop(&mut self)
    {
        #[cfg(feature = "std")]
        if let Some(flag) = self.poison_flag
            && std::thread::panicking()
        {
            flag.store(true, Ordering::Release);
        }
        self.lock.free_share(&self.meta);
    }
}

impl<'a, T, L, M> Deref for ShGuard<'a, T, L, M>
where L: SharingPolicy<Meta = M>
{
    type Target = T;
    fn deref(&self) -> &Self::Target
    {
        unsafe { self.data.as_ref_unchecked() }
    }
}
