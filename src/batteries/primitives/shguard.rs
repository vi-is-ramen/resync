//! A shared RAII guard that provides shared (read) access to the protected
//! data.

use crate::api::{LockPolicy, PoisonPolicy, SharingPolicy};
use core::ops::Deref;

/// A shared RAII guard that provides shared (read) access to the protected
/// data.
#[allow(missing_debug_implementations)]
pub struct ShGuard<'a, T, L, P, M = <L as LockPolicy>::Meta>
where
    L: SharingPolicy<Meta = M>,
    P: PoisonPolicy,
{
    data:        *const T,
    lock:        &'a L,
    meta:        M,
    poison_flag: &'a P,
}

unsafe impl<'a, T, L, P, M> core::marker::Send for ShGuard<'a, T, L, P, M>
where
    T: Send,
    L: SharingPolicy<Meta = M> + Send,
    P: PoisonPolicy,
    M: Send,
{
}

impl<'a, T, L, P, M> ShGuard<'a, T, L, P, M>
where
    L: SharingPolicy<Meta = M>,
    P: PoisonPolicy,
{
    /// Creates a new shared guard.
    pub fn new(data: *const T, lock: &'a L, meta: M, poison_flag: &'a P)
    -> Self
    {
        Self {
            data,
            lock,
            meta,
            poison_flag,
        }
    }
}

impl<'a, T, L, P, M> core::fmt::Debug for ShGuard<'a, T, L, P, M>
where
    T: core::fmt::Debug,
    L: SharingPolicy<Meta = M>,
    P: PoisonPolicy,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        let inner = unsafe { self.data.as_ref_unchecked() };
        <T as core::fmt::Debug>::fmt(inner, f)
    }
}

impl<'a, T, L, P, M> core::ops::Drop for ShGuard<'a, T, L, P, M>
where
    L: SharingPolicy<Meta = M>,
    P: PoisonPolicy,
{
    fn drop(&mut self)
    {
        P::on_drop(self.poison_flag);
        self.lock.free_share(&self.meta);
    }
}

impl<'a, T, L, P, M> Deref for ShGuard<'a, T, L, P, M>
where
    L: SharingPolicy<Meta = M>,
    P: PoisonPolicy,
{
    type Target = T;
    fn deref(&self) -> &Self::Target
    {
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L, P, M> crate::api::Guard<T> for ShGuard<'a, T, L, P, M>
where
    L: SharingPolicy<Meta = M>,
    P: PoisonPolicy,
{
}
