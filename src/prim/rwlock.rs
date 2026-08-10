//! A readers-writer lock that uses a spin strategy.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{ISpin, SpinResult};

/// A constant representing that a writer holds the lock.
///
/// We use `usize::MAX` to represent the writer state. This leaves
/// `0` through `usize::MAX - 1` available to track the number of
/// concurrent readers.
const WRITER: usize = usize::MAX;

/// A readers-writer lock primitive that protects a value of type `T`.
///
/// The `RwLock` is parameterised by:
/// - `T`: the data being protected.
/// - `S`: the spin strategy (must implement [`ISpin`]) used while waiting for
///   the lock.
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
/// underlying spin strategy reports an abort.
#[allow(missing_debug_implementations)]
pub struct RwLock<T, S: ISpin = crate::spin::DefaultSpin>
{
    inner:   UnsafeCell<T>,
    counter: AtomicUsize,
    spin:    S,
}

impl<T: core::default::Default, S: ISpin> core::default::Default
    for RwLock<T, S>
{
    fn default() -> Self
    {
        Self {
            inner:   UnsafeCell::new(T::default()),
            counter: AtomicUsize::new(0),
            spin:    S::default(),
        }
    }
}

/// A guard that provides immutable access to the protected data.
///
/// The guard releases the read lock when dropped.
#[allow(missing_debug_implementations)]
pub struct RwRef<'a, T, S: ISpin>
{
    data: *const T,
    lock: &'a RwLock<T, S>,
}

/// A guard that provides mutable access to the protected data.
///
/// The guard releases the write lock when dropped.
#[allow(missing_debug_implementations)]
pub struct RwMut<'a, T, S: ISpin>
{
    data: *mut T,
    lock: &'a RwLock<T, S>,
}

impl<'a, T, S: ISpin> core::ops::Drop for RwMut<'a, T, S>
{
    /// Releases the write lock when the guard goes out of scope.
    fn drop(&mut self)
    {
        self.lock.free_mut();
    }
}

impl<'a, T, S: ISpin> core::ops::Drop for RwRef<'a, T, S>
{
    /// Releases the read lock when the guard goes out of scope.
    fn drop(&mut self)
    {
        self.lock.free();
    }
}

impl<'a, T, S: ISpin> core::ops::Deref for RwRef<'a, T, S>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        // Safety: the guard holds the read lock, so no mutable access exists.
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, S: ISpin> core::ops::Deref for RwMut<'a, T, S>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        // Safety: the guard holds the write lock, so no other access exists.
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, S: ISpin> core::ops::DerefMut for RwMut<'a, T, S>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        // Safety: the guard holds the write lock, so no other access exists.
        unsafe { self.data.as_mut_unchecked() }
    }
}

impl<T, S: ISpin> RwLock<T, S>
{
    /// Creates a new `RwLock` protecting the given `value`.
    ///
    /// The spin strategy is initialised with its [`core::default::Default`]
    /// implementation.
    pub fn new(value: T) -> Self
    {
        Self {
            inner:   UnsafeCell::new(value),
            counter: AtomicUsize::new(0),
            spin:    S::default(),
        }
    }

    /// Attempts to acquire a read lock without blocking.
    ///
    /// # Returns
    /// - [`Some`] – the read lock was acquired.
    /// - [`None`] – the lock is currently held by a writer.
    pub fn try_read(&self) -> Option<RwRef<'_, T, S>>
    {
        loop
        {
            let state = self.counter.load(Ordering::Relaxed);
            if state == WRITER
            {
                return None;
            }

            if self
                .counter
                .compare_exchange_weak(
                    state,
                    state + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Some(RwRef {
                    data: self.inner.get(),
                    lock: self,
                });
            }
        }
    }

    /// Acquires a read lock, spinning until it is acquired or an abort occurs.
    ///
    /// # Returns
    /// - [`Some`] – the read lock was acquired.
    /// - [`None`] – the spin strategy reported an abort.
    pub fn read(&self) -> Option<RwRef<'_, T, S>>
    {
        loop
        {
            let state = self.counter.load(Ordering::Relaxed);
            if state == WRITER
            {
                match self.spin.spin()
                {
                    SpinResult::Ok => continue,
                    SpinResult::Abort => return None,
                }
            }

            if self
                .counter
                .compare_exchange_weak(
                    state,
                    state + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Some(RwRef {
                    data: self.inner.get(),
                    lock: self,
                });
            }

            if let SpinResult::Abort = self.spin.spin()
            {
                return None;
            }
        }
    }

    /// Attempts to acquire a write lock without blocking.
    ///
    /// # Returns
    /// - [`Some`] – the write lock was acquired.
    /// - [`None`] – the lock is currently held by readers or another writer.
    pub fn try_write(&self) -> Option<RwMut<'_, T, S>>
    {
        loop
        {
            let state = self.counter.load(Ordering::Relaxed);
            if state != 0
            {
                return None;
            }

            if self
                .counter
                .compare_exchange_weak(
                    0,
                    WRITER,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Some(RwMut {
                    data: self.inner.get(),
                    lock: self,
                });
            }
        }
    }

    /// Acquires a write lock, spinning until it is acquired or an abort occurs.
    ///
    /// # Returns
    /// - [`Some`] – the write lock was acquired.
    /// - [`None`] – the spin strategy reported an abort.
    pub fn write(&self) -> Option<RwMut<'_, T, S>>
    {
        loop
        {
            if self
                .counter
                .compare_exchange_weak(
                    0,
                    WRITER,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Some(RwMut {
                    data: self.inner.get(),
                    lock: self,
                });
            }

            match self.spin.spin()
            {
                SpinResult::Ok => continue,
                SpinResult::Abort => return None,
            }
        }
    }

    /// Releases a read lock.
    ///
    /// This method is not idempotent and must only be called by the `RwRef`
    /// guard.
    fn free(&self)
    {
        self.counter.fetch_sub(1, Ordering::Release);
    }

    /// Releases a write lock.
    ///
    /// This method is not idempotent and must only be called by the `RwMut`
    /// guard.
    fn free_mut(&self)
    {
        self.counter.store(0, Ordering::Release);
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
        let lock = RwLock::<u32, Busy>::new(42);
        let guard = lock.read().unwrap();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn rwlock_write_and_read()
    {
        let lock = RwLock::<u32, Busy>::new(0);
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
        let lock = RwLock::<u32, Busy>::new(42);
        let g1 = lock.read().unwrap();
        let g2 = lock.read().unwrap();
        assert_eq!(*g1, 42);
        assert_eq!(*g2, 42);
    }

    #[test]
    fn rwlock_try_write_fails_when_read()
    {
        let lock = RwLock::<u32, Busy>::new(42);
        let _g = lock.read().unwrap();
        assert!(lock.try_write().is_none());
    }

    #[test]
    fn rwlock_try_read_fails_when_written()
    {
        let lock = RwLock::<u32, Busy>::new(42);
        let _g = lock.write().unwrap();
        assert!(lock.try_read().is_none());
    }

    #[test]
    fn rwlock_default_works()
    {
        let lock = RwLock::<u32, Busy>::default();
        let guard = lock.read().unwrap();
        assert_eq!(*guard, 0);
    }
}
