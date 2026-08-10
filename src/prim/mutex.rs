//! A mutual exclusion primitive that uses a lock and a spin strategy.

use core::cell::UnsafeCell;

use crate::{LockResult, SpinResult};

/// A mutual exclusion (mutex) primitive that protects a value of type `T`.
///
/// The mutex is parameterised by:
/// - `L`: the lock implementation (must implement [`crate::ILock`]).
/// - `S`: the spin strategy (must implement [`crate::ISpin`]) used while
///   waiting for the lock.
///
/// # Examples
/// ```
/// use resync::Mutex;
/// let mutex: Mutex<u32> = Mutex::new(42u32);
/// {
///     let guard = mutex.lock().unwrap();
///     assert_eq!(*guard, 42);
/// }
/// ```
///
/// # Errors
/// The [`Mutex::lock`] method returns `None` if the underlying lock
/// reports an abort.
#[allow(missing_debug_implementations)]
pub struct Mutex<
    T,
    L: crate::ILock = crate::lock::Atomic,
    S: crate::ISpin = crate::spin::DefaultSpin,
> {
    inner: UnsafeCell<T>,
    lock:  L,
    spin:  S,
}

impl<T: core::default::Default, L: crate::ILock, S: crate::ISpin>
    core::default::Default for Mutex<T, L, S>
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
pub struct MutexGuard<'a, T, L: crate::ILock>
{
    data: *mut T,
    lock: &'a L,
}

impl<'a, T, L: crate::ILock> core::ops::Drop for MutexGuard<'a, T, L>
{
    /// Releases the lock when the guard goes out of scope.
    fn drop(&mut self)
    {
        self.lock.free();
    }
}

impl<'a, T, L: crate::ILock> core::ops::Deref for MutexGuard<'a, T, L>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        // Safety: the guard holds the lock, so no other mutable access exists.
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L: crate::ILock> core::ops::DerefMut for MutexGuard<'a, T, L>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        // Safety: the guard holds the lock, so no other mutable access exists.
        unsafe { self.data.as_mut_unchecked() }
    }
}

impl<T, L: crate::ILock, S: crate::ISpin> Mutex<T, L, S>
{
    /// Creates a new mutex protecting the given `value`.
    ///
    /// The lock and spin strategy are initialised with their
    /// [`core::default::Default`] implementations.
    pub fn new(value: T) -> Self
    {
        Self {
            inner: UnsafeCell::new(value),
            lock:  L::default(),
            spin:  S::default(),
        }
    }

    /// Acquires the mutex but not blocking until the lock is acquired or an
    /// abort occurs: returns Option instead.
    ///
    /// # Returns
    /// - [`Some`] – the lock was acquired and the guard grants access to the
    ///   protected data.
    /// - [`None`] – the underlying lock reported an abort or mutex is locked.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T, L>>
    {
        match self.lock.try_lock()
        {
            LockResult::Abort => return None,
            LockResult::Done =>
            {
                return Some(MutexGuard {
                    data: self.inner.get(),
                    lock: &self.lock,
                });
            },
            LockResult::Fail => return None,
        }
    }

    /// Acquires the mutex, blocking until the lock is acquired or an abort
    /// occurs.
    ///
    /// # Returns
    /// - [`Some`] – the lock was acquired and the guard grants access to the
    ///   protected data.
    /// - [`None`] – the underlying lock reported an abort (unrecoverable
    ///   error).
    ///
    /// # Panics
    /// This method does not panic, but may loop forever if the lock never
    /// becomes available (though the spin strategy will eventually yield or
    /// pause).
    pub fn lock(&self) -> Option<MutexGuard<'_, T, L>>
    {
        loop
        {
            match self.lock.try_lock()
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
    use crate::{ILock, ISpin};

    // A mock lock that always returns Abort.
    #[derive(Default)]
    struct AbortLock;
    impl ILock for AbortLock
    {
        fn try_lock(&self) -> LockResult
        {
            LockResult::Abort
        }
        fn free(&self) {}
    }

    // A mock spin that aborts on first call (returns Abort).
    #[derive(Default)]
    struct AbortSpin;
    impl ISpin for AbortSpin
    {
        fn spin(&self) -> SpinResult
        {
            SpinResult::Abort
        }
    }

    #[test]
    fn mutex_new_and_lock_unlock()
    {
        let mutex = Mutex::<u32, Atomic, Busy>::new(42);
        {
            let guard = mutex.lock().unwrap();
            assert_eq!(*guard, 42);
            // Guard dropped -> lock freed.
        }
        // Lock again to ensure it's free.
        let guard2 = mutex.lock().unwrap();
        assert_eq!(*guard2, 42);
    }

    #[test]
    fn mutex_default_works()
    {
        let mutex = Mutex::<u32, Atomic, Busy>::default();
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, 0); // u32 default is 0
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
        // Hold lock.
        let guard = mutex.lock().unwrap();
        // Drop hold.
        drop(guard);
        let _ = mutex.lock().unwrap(); // should succeed
    }

    #[test]
    fn mutex_returns_none_on_lock_abort()
    {
        // Use a lock that aborts.
        let mutex = Mutex::<u32, AbortLock, Busy>::new(5);
        assert!(mutex.lock().is_none());
    }

    #[test]
    fn mutex_returns_none_on_spin_abort()
    {
        // Use a spin that aborts, but lock must fail repeatedly to trigger
        // spin. We need a lock that returns Fail, not Done, to go into
        // spin loop.
        #[derive(Default)]
        struct FailLock;
        impl ILock for FailLock
        {
            fn try_lock(&self) -> LockResult
            {
                LockResult::Fail
            }
            fn free(&self) {}
        }
        let mutex = Mutex::<u32, FailLock, AbortSpin>::new(5);
        assert!(mutex.lock().is_none());
    }

    // Test that mutex works with the default spin (which is Os if std, else
    // Busy).
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
