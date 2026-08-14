//! A mutual exclusion primitive that uses a lock and a spin strategy.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use crate::{ILock, ISpin, LockResult, SpinResult};

/// A mutual exclusion (mutex) primitive that protects a value of type `T`.
///
/// The mutex is parameterised by:
/// - `T`: the protected data type.
/// - `L`: the lock implementation (must implement [`ILock`]).
/// - `S`: the spin strategy (must implement [`ISpin`]).
///
/// The lock implementation decides when to park based on the iteration
/// count passed to [`ILock::try_lock`].
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
///
/// The guard releases the lock when dropped.
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
    /// - [`Some`] – the lock was acquired.
    /// - [`None`] – the lock is held or an abort occurred.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T, L>>
    {
        match self.lock.try_lock(0)
        {
            LockResult::Done => Some(MutexGuard {
                data: self.inner.get(),
                lock: &self.lock,
            }),
            _ => None,
        }
    }

    /// Acquires the mutex, blocking until available or an abort occurs.
    ///
    /// The iteration count is passed to [`ILock::try_lock`], allowing the
    /// lock implementation to decide when to park the current thread.
    pub fn lock(&self) -> Option<MutexGuard<'_, T, L>>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;

            match self.lock.try_lock(iterations)
            {
                LockResult::Abort => return None,
                LockResult::Done =>
                {
                    return Some(MutexGuard {
                        data: self.inner.get(),
                        lock: &self.lock,
                    });
                },
                LockResult::Fail => match self.spin.spin()
                {
                    SpinResult::Ok => continue,
                    SpinResult::Abort => return None,
                },
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

    #[test]
    fn mutex_default_works()
    {
        let mutex = Mutex::<u32, Atomic, Busy>::default();
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, 0);
    }

    #[test]
    fn mutex_guard_deref_mut()
    {
        let mutex = Mutex::<u32, Atomic, Busy>::new(10);
        let mut guard = mutex.lock().unwrap();
        *guard = 20;
        assert_eq!(*guard, 20);
        drop(guard);
        let guard2 = mutex.lock().unwrap();
        assert_eq!(*guard2, 20);
    }

    #[test]
    fn mutex_guard_drop_releases_lock()
    {
        let mutex = Mutex::<u32, Atomic, Busy>::new(0);
        let guard = mutex.lock().unwrap();
        drop(guard);
        let _ = mutex.lock().unwrap();
    }

    #[test]
    fn mutex_with_default_spin()
    {
        let mutex = Mutex::<u32>::new(100);
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, 100);
        drop(guard);
        let guard2 = mutex.lock().unwrap();
        assert_eq!(*guard2, 100);
    }
}
