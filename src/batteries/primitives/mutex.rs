//! A mutual exclusion primitive that uses a lock and a spin strategy.

use crate::LockStatus;
use crate::traits::{LockPolicy, RetryPolicy};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// A mutual exclusion (mutex) primitive that protects a value of type `T`.
#[allow(missing_debug_implementations)]
pub struct Mutex<T, L = crate::lock::Os, R = crate::retry::Yield>
where
    L: LockPolicy,
    R: RetryPolicy,
{
    inner: UnsafeCell<T>,
    lock:  L,
    retry: R,
}

unsafe impl<T, L, R> core::marker::Sync for Mutex<T, L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
}

unsafe impl<T, L, R> core::marker::Send for Mutex<T, L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
}

impl<T, L, R> core::default::Default for Mutex<T, L, R>
where
    T: Default,
    L: LockPolicy,
    R: RetryPolicy,
{
    fn default() -> Self
    {
        Self {
            inner: UnsafeCell::new(T::default()),
            lock:  L::default(),
            retry: R::default(),
        }
    }
}

/// A guard that provides mutable access to the protected data.
#[allow(missing_debug_implementations)]
pub struct MutexGuard<'a, T, L: LockPolicy>
{
    data: *mut T,
    lock: &'a L,
}

impl<'a, T, L: LockPolicy> core::ops::Drop for MutexGuard<'a, T, L>
{
    fn drop(&mut self)
    {
        unsafe { self.lock.free() };
    }
}

impl<'a, T, L: LockPolicy> Deref for MutexGuard<'a, T, L>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L: LockPolicy> DerefMut for MutexGuard<'a, T, L>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        unsafe { self.data.as_mut_unchecked() }
    }
}

impl<T, L: LockPolicy, S: RetryPolicy> Mutex<T, L, S>
{
    /// Creates a new mutex protecting the given `value`.
    pub fn new(value: T) -> Self
    {
        Self {
            inner: UnsafeCell::new(value),
            lock:  L::default(),
            retry: S::default(),
        }
    }

    /// Attempts to acquire the mutex without blocking.
    ///
    /// # Returns
    /// - `Some(guard)`: lock acquired
    /// - `None`: lock is held or error occurred
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T, L>>
    {
        match unsafe { self.lock.try_lock(0) }
        {
            Ok(LockStatus::Done) => Some(MutexGuard {
                data: self.inner.get(),
                lock: &self.lock,
            }),
            _ => None,
        }
    }

    /// Acquires the mutex, blocking until available.
    ///
    /// # Returns
    /// - `Some(guard)`: lock acquired
    /// - `None`: unrecoverable error or spin aborted
    pub fn lock(&self) -> Option<MutexGuard<'_, T, L>>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;

            match unsafe { self.lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done) =>
                {
                    return Some(MutexGuard {
                        data: self.inner.get(),
                        lock: &self.lock,
                    });
                },
                Ok(LockStatus::Fail) =>
                {
                    if self.retry.retry(iterations).is_err()
                    {
                        return None;
                    }
                },
                Err(_) => return None,
            }
        }
    }
}
