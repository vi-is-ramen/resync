use crate::traits::{LockPolicy, PoisonPolicy};
use core::ops::{Deref, DerefMut};

/// An exclusive RAII guard that provides mutable access to the protected data.
///
/// When this guard is dropped, the underlying lock is automatically released
/// via the [`LockPolicy::free`] method. The associated [`PoisonPolicy`] is
/// also notified to check for thread panics and potentially mark the lock
/// as poisoned.
pub struct ExGuard<'a, T, L, P, M = <L as LockPolicy>::Meta>
where
    L: LockPolicy<Meta = M>,
    P: PoisonPolicy,
{
    data:        *mut T,
    lock:        &'a L,
    meta:        M,
    poison_flag: &'a P::State,
}

unsafe impl<'a, T, L, P, M> core::marker::Send for ExGuard<'a, T, L, P, M>
where
    T: Send,
    L: LockPolicy<Meta = M> + Send,
    P: PoisonPolicy,
    M: Send,
{
}

impl<'a, T, L, P, M> ExGuard<'a, T, L, P, M>
where
    L: LockPolicy<Meta = M>,
    P: PoisonPolicy,
{
    /// Creates a new guard.
    pub fn new(
        data: *mut T,
        lock: &'a L,
        meta: M,
        poison_flag: &'a P::State,
    ) -> Self
    {
        Self {
            data,
            lock,
            meta,
            poison_flag,
        }
    }
}

impl<'a, T, L, P, M> ExGuard<'a, T, L, P, M>
where
    T: Default,
    L: LockPolicy<Meta = M>,
    P: PoisonPolicy,
{
    /// Takes the value out of the guarded data, leaving `Default::default()`
    /// in its place, and releases the lock.
    pub fn take(self) -> T
    {
        let inner = unsafe { self.data.as_mut_unchecked() };
        core::mem::take(inner)
    }
}

impl<'a, T, L, P, M> ExGuard<'a, T, L, P, M>
where
    L: LockPolicy<Meta = M>,
    P: PoisonPolicy,
{
    /// Exchanges the protected value with a new value, returning the old value.
    pub fn exchange(self, value: T) -> T
    {
        let inner = unsafe { self.data.as_mut_unchecked() };
        core::mem::replace(inner, value)
    }
}

impl<'a, T, L, P, M> core::fmt::Debug for ExGuard<'a, T, L, P, M>
where
    T: core::fmt::Debug,
    L: LockPolicy<Meta = M>,
    P: PoisonPolicy,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        let inner = unsafe { self.data.as_ref_unchecked() };
        <T as core::fmt::Debug>::fmt(inner, f)
    }
}

impl<'a, T, L, P, M> core::ops::Drop for ExGuard<'a, T, L, P, M>
where
    L: LockPolicy<Meta = M>,
    P: PoisonPolicy,
{
    fn drop(&mut self)
    {
        P::on_drop(self.poison_flag);
        unsafe { self.lock.free(&self.meta) };
    }
}

impl<'a, T, L, P, M> Deref for ExGuard<'a, T, L, P, M>
where
    L: LockPolicy<Meta = M>,
    P: PoisonPolicy,
{
    type Target = T;
    fn deref(&self) -> &Self::Target
    {
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L, P, M> DerefMut for ExGuard<'a, T, L, P, M>
where
    L: LockPolicy<Meta = M>,
    P: PoisonPolicy,
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        unsafe { self.data.as_mut_unchecked() }
    }
}
