//! A mutual exclusion primitive that composes a lock policy and a retry policy.
//!
//! This module provides the [`Mutex`] struct, which is the primary high-level
//! synchronization primitive in `resync`. Unlike standard library mutexes that
//! hardcode their acquisition and waiting strategies, `resync::Mutex` is
//! generic over three parameters:
//!
//! - `T`: The type of the data being protected.
//! - `L`: The [`LockPolicy`](crate::traits::LockPolicy) used to acquire and
//!   release the lock (e.g., [`Atomic`](crate::lock::Atomic) or
//!   [`Os`](crate::lock::Os)).
//! - `R`: The [`RetryPolicy`](crate::traits::RetryPolicy) used to wait when the
//!   lock is contended (e.g., [`Busy`](crate::retry::Busy) or
//!   [`Yield`](crate::retry::Yield)).
//!
//! # Examples
//!
//! ```rust
//! # use resync::Mutex;
//! let mutex = Mutex::<i32>::new(42);
//!
//! {
//!     let mut guard = mutex.lock().unwrap();
//!     *guard += 1;
//!     assert_eq!(*guard, 43);
//! } // Guard is dropped, lock is automatically released
//! ```

use crate::LockStatus;
use crate::traits::{LockPolicy, RetryPolicy};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// A mutual exclusion (mutex) primitive that protects a value of type `T`.
///
/// The mutex uses a lock policy `L` to manage the underlying lock state, and
/// a retry policy `R` to determine how to wait when the lock is already held
/// by another thread.
///
/// By default, it uses [`crate::lock::Os`] as the lock policy and
/// [`crate::retry::Yield`] as the retry policy (when the `std` feature is
/// enabled).
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct Mutex<T, L = crate::lock::Os, R = crate::retry::Yield>
where
    L: LockPolicy,
    R: RetryPolicy,
{
    inner: UnsafeCell<T>,
    lock:  L,
    retry: R,
}

// SAFETY:
// The mutex ensures exclusive access to `T` via the lock policy `L`. As long as
// `T` is `Send`, the mutex itself can be safely shared across threads.
unsafe impl<T, L, R> core::marker::Sync for Mutex<T, L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
}

// SAFETY:
// The mutex can be safely moved between threads as long as `T` is `Send`.
unsafe impl<T: core::marker::Send, L, R> core::marker::Send for Mutex<T, L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
}

/// A RAII guard that provides mutable access to the protected data.
///
/// When this guard is dropped, the underlying lock is automatically released
/// via the [`LockPolicy::free`] method.
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
        // SAFETY:
        // The guard's existence guarantees that the lock is currently held by
        // the current thread.
        unsafe { self.lock.free() };
    }
}

impl<'a, T, L: LockPolicy> Deref for MutexGuard<'a, T, L>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        // SAFETY:
        // The guard guarantees exclusive access to the data.
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L: LockPolicy> DerefMut for MutexGuard<'a, T, L>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        // SAFETY:
        // The guard guarantees exclusive access to the data.
        unsafe { self.data.as_mut_unchecked() }
    }
}

impl<T, L: LockPolicy, S: RetryPolicy> Mutex<T, L, S>
{
    /// Creates a new mutex protecting the given `value`.
    ///
    /// The lock and retry policies are initialized using their `Default`
    /// implementations.
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
    /// This method calls [`LockPolicy::try_lock`] exactly once. If the lock
    /// is currently held by another thread, or if an unrecoverable error
    /// occurs, it returns `None`.
    ///
    /// # Returns
    ///
    /// - `Some(guard)`: The lock was successfully acquired.
    /// - `None`: The lock is currently held, or an error occurred.
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

    /// Acquires the mutex, blocking the current thread until it is available.
    ///
    /// This method repeatedly calls [`LockPolicy::try_lock`]. If the lock is
    /// not immediately available, it calls [`RetryPolicy::retry`] to wait
    /// (e.g., by spinning or yielding) before trying again.
    ///
    /// # Returns
    ///
    /// - `Some(guard)`: The lock was successfully acquired.
    /// - `None`: The retry policy aborted (e.g., due to a timeout or fatal
    ///   error), or an unrecoverable error occurred in the lock policy.
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
