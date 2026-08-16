use crate::traits::LockPolicy;
use core::ops::{Deref, DerefMut};
#[cfg(feature = "std")]
use core::sync::atomic::{AtomicBool, Ordering};

/// An exclusive RAII guard that provides mutable access to the protected data.
///
/// When this guard is dropped, the underlying lock is automatically released
/// via the [`LockPolicy::free`] method. If the `std` feature is enabled and
/// the current thread is panicking, the associated lock will be marked as
/// poisoned.
pub struct ExGuard<'a, T, L, M = <L as LockPolicy>::Meta>
where L: LockPolicy<Meta = M>
{
    data:        *mut T,
    lock:        &'a L,
    meta:        M,
    #[cfg(feature = "std")]
    poison_flag: Option<&'a AtomicBool>,
}

unsafe impl<'a, T, L, M> core::marker::Send for ExGuard<'a, T, L, M>
where
    T: Send,
    L: LockPolicy<Meta = M> + Send,
    M: Send,
{
}

impl<'a, T, L, M> ExGuard<'a, T, L, M>
where L: LockPolicy<Meta = M>
{
    /// Creates a new guard (with `std` feature enabled).
    #[cfg(feature = "std")]
    pub fn new(
        data: *mut T,
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

    /// Creates a new guard (without `std` feature).
    #[cfg(not(feature = "std"))]
    pub fn new(data: *mut T, lock: &'a L, meta: M) -> Self
    {
        Self { data, lock, meta }
    }
}

impl<'a, T, L, M> ExGuard<'a, T, L, M>
where
    T: Default,
    L: LockPolicy<Meta = M>,
{
    /// Takes the value out of the guarded data, leaving `Default::default()`
    /// in its place, and releases the lock.
    pub fn take(self) -> T
    {
        let inner = unsafe { self.data.as_mut_unchecked() };
        core::mem::take(inner)
    }
}

impl<'a, T, L, M> ExGuard<'a, T, L, M>
where L: LockPolicy<Meta = M>
{
    /// Exchanges the protected value with a new value, returning the old value.
    pub fn exchange(self, value: T) -> T
    {
        let inner = unsafe { self.data.as_mut_unchecked() };
        core::mem::replace(inner, value)
    }
}

impl<'a, T, L, M> core::fmt::Debug for ExGuard<'a, T, L, M>
where
    T: core::fmt::Debug,
    L: LockPolicy<Meta = M>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        let inner = unsafe { self.data.as_ref_unchecked() };
        <T as core::fmt::Debug>::fmt(inner, f)
    }
}

impl<'a, T, L, M> core::ops::Drop for ExGuard<'a, T, L, M>
where L: LockPolicy<Meta = M>
{
    fn drop(&mut self)
    {
        #[cfg(feature = "std")]
        if let Some(flag) = self.poison_flag
            && std::thread::panicking()
        {
            flag.store(true, Ordering::Release);
        }
        unsafe { self.lock.free(&self.meta) };
    }
}

impl<'a, T, L, M> Deref for ExGuard<'a, T, L, M>
where L: LockPolicy<Meta = M>
{
    type Target = T;
    fn deref(&self) -> &Self::Target
    {
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L, M> DerefMut for ExGuard<'a, T, L, M>
where L: LockPolicy<Meta = M>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        unsafe { self.data.as_mut_unchecked() }
    }
}
