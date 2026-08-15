use core::ops::{Deref, DerefMut};

use crate::traits::LockPolicy;

/// An exclusive RAII guard that provides mutable access to the protected data.
///
/// When this guard is dropped, the underlying lock is automatically released
/// via the [`LockPolicy::free`] method.
pub struct ExGuard<'a, T, L>
where L: LockPolicy
{
    data: *mut T,
    lock: &'a L,
}

impl<'a, T, L> ExGuard<'a, T, L>
where L: LockPolicy
{
    /// Creates a new guard.
    pub fn new(data: *mut T, lock: &'a L) -> Self
    {
        Self { data, lock }
    }
}

impl<'a, T, L> ExGuard<'a, T, L>
where
    T: Default,
    L: LockPolicy,
{
    /// Takes the value out of the guarded data, leaving `Default::default()`
    /// in its place, and releases the lock.
    pub fn take(self) -> T
    {
        let inner = unsafe { self.data.as_mut_unchecked() };
        core::mem::take(inner)
    }
}

impl<'a, T, L> ExGuard<'a, T, L>
where L: LockPolicy
{
    /// Exchanges the protected value with a new value, returning the old value.
    ///
    /// This consumes the guard, sets the new value, and releases the lock.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use resync::Mutex;
    /// let mutex = Mutex::<i32>::new(42);
    /// let guard = mutex.lock().unwrap();
    /// let old = guard.exchange(100);
    /// assert_eq!(old, 42);
    /// // The mutex now contains 100
    /// ```
    pub fn exchange(self, value: T) -> T
    {
        let inner = unsafe { self.data.as_mut_unchecked() };
        core::mem::replace(inner, value)
    }
}

impl<'a, T, L> core::fmt::Debug for ExGuard<'a, T, L>
where
    T: core::fmt::Debug,
    L: LockPolicy,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        let inner = unsafe { self.data.as_ref_unchecked() };
        <T as core::fmt::Debug>::fmt(inner, f)
    }
}

impl<'a, T, L> core::ops::Drop for ExGuard<'a, T, L>
where L: LockPolicy
{
    fn drop(&mut self)
    {
        unsafe { self.lock.free() };
    }
}

impl<'a, T, L> Deref for ExGuard<'a, T, L>
where L: LockPolicy
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L> DerefMut for ExGuard<'a, T, L>
where L: LockPolicy
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        unsafe { self.data.as_mut_unchecked() }
    }
}
