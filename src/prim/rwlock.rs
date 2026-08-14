//! A readers-writer lock that uses a spin strategy.

use core::cell::UnsafeCell;

use crate::{IShare, ISpin, LockResult, SpinResult};

/// A readers-writer lock primitive that protects a value of type `T`.
///
/// The `RwLock` is parameterised by:
/// - `T`: the data being protected.
/// - `L`: the shared lock implementation (must implement [`IShare`]).
/// - `S`: the spin strategy (must implement [`ISpin`]) used while waiting.
///
/// # Examples
/// ```
/// use resync::RwLock;
/// let lock: RwLock<u32> = RwLock::new(42u32);
/// {
///     let guard = lock.read().unwrap();
///     assert_eq!(*guard, 42);
/// }
/// {
///     let mut guard = lock.write().unwrap();
///     *guard += 1;
/// }
/// ```
///
/// # Errors
/// The [`RwLock::read`] and [`RwLock::write`] methods return `None` if the
/// underlying lock reports `Abort` (unrecoverable error) or if the spin
/// strategy aborts.
#[allow(missing_debug_implementations)]
pub struct RwLock<
    T,
    L: IShare = crate::share::Atomic,
    S: ISpin = crate::spin::DefaultSpin,
> {
    inner: UnsafeCell<T>,
    lock:  L,
    spin:  S,
}

unsafe impl<T, L: IShare, S: ISpin> core::marker::Sync for RwLock<T, L, S> {}
unsafe impl<T, L: IShare, S: ISpin> core::marker::Send for RwLock<T, L, S> {}

impl<T: core::default::Default, L: IShare, S: ISpin> core::default::Default
    for RwLock<T, L, S>
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

/// A guard that provides immutable access to the protected data.
/// The guard releases the read lock when dropped.
#[allow(missing_debug_implementations)]
pub struct RwRef<'a, T, L: IShare, S: ISpin>
{
    data: *const T,
    lock: &'a RwLock<T, L, S>,
}

/// A guard that provides mutable access to the protected data.
/// The guard releases the write lock when dropped.
#[allow(missing_debug_implementations)]
pub struct RwMut<'a, T, L: IShare, S: ISpin>
{
    data: *mut T,
    lock: &'a RwLock<T, L, S>,
}

impl<'a, T, L: IShare, S: ISpin> core::ops::Drop for RwRef<'a, T, L, S>
{
    fn drop(&mut self)
    {
        self.lock.lock.free_read();
    }
}

impl<'a, T, L: IShare, S: ISpin> core::ops::Drop for RwMut<'a, T, L, S>
{
    fn drop(&mut self)
    {
        self.lock.lock.free_write();
    }
}

impl<'a, T, L: IShare, S: ISpin> core::ops::Deref for RwRef<'a, T, L, S>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        // Safety: the guard holds the read lock, so no mutable access exists.
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L: IShare, S: ISpin> core::ops::Deref for RwMut<'a, T, L, S>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        // Safety: the guard holds the write lock, so no other access exists.
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L: IShare, S: ISpin> core::ops::DerefMut for RwMut<'a, T, L, S>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        // Safety: the guard holds the write lock, so no other access exists.
        unsafe { self.data.as_mut_unchecked() }
    }
}

impl<T, L: IShare, S: ISpin> RwLock<T, L, S>
{
    /// Creates a new `RwLock` protecting the given `value`.
    pub fn new(value: T) -> Self
    {
        Self {
            inner: UnsafeCell::new(value),
            lock:  L::default(),
            spin:  S::default(),
        }
    }

    /// Attempts to acquire a read lock without blocking.
    ///
    /// # Returns
    /// - `Some` – the read lock was acquired.
    /// - `None` – the lock is currently held by a writer, or an abort occurred.
    pub fn try_read(&self) -> Option<RwRef<'_, T, L, S>>
    {
        match self.lock.try_read()
        {
            LockResult::Done => Some(RwRef {
                data: self.inner.get(),
                lock: self,
            }),
            _ => None,
        }
    }

    /// Acquires a read lock, spinning until it is acquired or an abort occurs.
    pub fn read(&self) -> Option<RwRef<'_, T, L, S>>
    {
        loop
        {
            match self.lock.try_read()
            {
                LockResult::Done =>
                {
                    return Some(RwRef {
                        data: self.inner.get(),
                        lock: self,
                    });
                },
                LockResult::Abort => return None,
                LockResult::Fail => match self.spin.spin()
                {
                    SpinResult::Ok => continue,
                    SpinResult::Abort => return None,
                },
            }
        }
    }

    /// Attempts to acquire a write lock without blocking.
    pub fn try_write(&self) -> Option<RwMut<'_, T, L, S>>
    {
        match self.lock.try_write()
        {
            LockResult::Done => Some(RwMut {
                data: self.inner.get(),
                lock: self,
            }),
            _ => None,
        }
    }

    /// Acquires a write lock, spinning until it is acquired or an abort occurs.
    pub fn write(&self) -> Option<RwMut<'_, T, L, S>>
    {
        loop
        {
            match self.lock.try_write()
            {
                LockResult::Done =>
                {
                    return Some(RwMut {
                        data: self.inner.get(),
                        lock: self,
                    });
                },
                LockResult::Abort => return None,
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
    use crate::spin::Busy;

    #[test]
    fn rwlock_new_and_read()
    {
        let lock = RwLock::<u32, crate::share::Atomic, Busy>::new(42);
        let guard = lock.read().unwrap();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn rwlock_write_and_read()
    {
        let lock = RwLock::<u32, crate::share::Atomic, Busy>::new(0);
        {
            let mut guard = lock.write().unwrap();
            *guard = 100;
        }
        let guard = lock.read().unwrap();
        assert_eq!(*guard, 100);
    }

    #[test]
    fn rwlock_multiple_readers()
    {
        let lock = RwLock::<u32, crate::share::Atomic, Busy>::new(42);
        let g1 = lock.read().unwrap();
        let g2 = lock.read().unwrap();
        assert_eq!(*g1, 42);
        assert_eq!(*g2, 42);
    }

    #[test]
    fn rwlock_try_write_fails_when_read()
    {
        let lock = RwLock::<u32, crate::share::Atomic, Busy>::new(42);
        let _g = lock.read().unwrap();
        assert!(lock.try_write().is_none());
    }

    #[test]
    fn rwlock_try_read_fails_when_written()
    {
        let lock = RwLock::<u32, crate::share::Atomic, Busy>::new(42);
        let _g = lock.write().unwrap();
        assert!(lock.try_read().is_none());
    }

    #[test]
    fn rwlock_default_works()
    {
        let lock = RwLock::<u32, crate::share::Atomic, Busy>::default();
        let guard = lock.read().unwrap();
        assert_eq!(*guard, 0);
    }
}
