//! A mutual exclusion primitive that uses a lock and a spin strategy.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use crate::{ILock, ISpin, LockStatus};

/// A mutual exclusion (mutex) primitive that protects a value of type `T`.
#[allow(missing_debug_implementations)]
pub struct Mutex<
    T,
    L: ILock = crate::lock::DefaultLock,
    S: ISpin = crate::spin::DefaultSpin,
> {
    inner: UnsafeCell<T>,
    lock:  L,
    spin:  S,
}

unsafe impl<T, L: ILock, S: ISpin> core::marker::Sync for Mutex<T, L, S> {}
unsafe impl<T, L: ILock, S: ISpin> core::marker::Send for Mutex<T, L, S> {}

impl<T: core::default::Default, L: ILock, S: ISpin> core::default::Default
    for Mutex<T, L, S>
{
    fn default() -> Self
    {
        Self {
            inner: UnsafeCell::new(T::default()),
            lock:  L::default(),
            spin:  S::default(),
        }
    }
}

/// A guard that provides mutable access to the protected data.
#[allow(missing_debug_implementations)]
pub struct MutexGuard<'a, T, L: ILock>
{
    data: *mut T,
    lock: &'a L,
}

impl<'a, T, L: ILock> core::ops::Drop for MutexGuard<'a, T, L>
{
    fn drop(&mut self)
    {
        self.lock.free();
    }
}

impl<'a, T, L: ILock> Deref for MutexGuard<'a, T, L>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L: ILock> DerefMut for MutexGuard<'a, T, L>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        unsafe { self.data.as_mut_unchecked() }
    }
}

impl<T, L: ILock, S: ISpin> Mutex<T, L, S>
{
    /// Creates a new mutex protecting the given `value`.
    pub fn new(value: T) -> Self
    {
        Self {
            inner: UnsafeCell::new(value),
            lock:  L::default(),
            spin:  S::default(),
        }
    }

    /// Attempts to acquire the mutex without blocking.
    ///
    /// # Returns
    /// - `Some(guard)`: lock acquired
    /// - `None`: lock is held or error occurred
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T, L>>
    {
        match self.lock.try_lock(0)
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

            match self.lock.try_lock(iterations)
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
                    if self.spin.spin().is_err()
                    {
                        return None;
                    }
                },
                Err(_) => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::lock::Atomic;
    use crate::spin::Busy;

    #[test]
    fn mutex_new_and_lock_unlock()
    {
        let mutex = Mutex::<u32, Atomic, Busy>::new(42);
        {
            let guard = mutex.lock().unwrap();
            assert_eq!(*guard, 42);
        }
        let guard2 = mutex.lock().unwrap();
        assert_eq!(*guard2, 42);
    }
}
