//! A shareable-exclusive (read-write) lock primitive.
//!
//! This module provides the [`Sharex`] struct, which allows multiple concurrent
//! readers or a single exclusive writer to access the protected data. It is
//! similar to [`std::sync::RwLock`], but built using `resync`'s composable
//! [`SharingPolicy`](crate::traits::SharingPolicy) and
//! [`RetryPolicy`](crate::traits::RetryPolicy) traits.
//!
//! # Examples
//!
//! ```rust
//! # use resync::Sharex;
//! let lock = Sharex::<i32>::new(5);
//!
//! // Multiple reader locks can be held concurrently.
//! {
//!     let r1 = lock.read().unwrap();
//!     let r2 = lock.read().unwrap();
//!     assert_eq!(*r1, 5);
//!     assert_eq!(*r2, 5);
//! } // Readers are dropped here.
//!
//! // Writer locks are exclusive.
//! {
//!     let mut w = lock.write().unwrap();
//!     *w += 1;
//!     assert_eq!(*w, 6);
//! } // Writer is dropped here.
//! ```

use crate::traits::{RetryPolicy, SharingPolicy};
use crate::{ExGuard, LockError, LockStatus, ShGuard, TryLockError};
use core::cell::UnsafeCell;

/// A shareable-exclusive (read-write) lock primitive that protects a value of
/// type `T`.
///
/// The `Sharex` lock uses a [`SharingPolicy`] `L` to manage the underlying lock
/// state, and a [`RetryPolicy`] `R` to determine how to wait when the lock is
/// contended.
///
/// By default, it uses [`crate::lock::Os`] as the sharing policy and
/// [`crate::retry::Yield`] as the retry policy (when the `std` feature is
/// enabled).
#[allow(missing_debug_implementations)]
pub struct Sharex<
    T,
    L = crate::lock::Shield<crate::lock::Os>,
    R = crate::retry::Yield,
> where
    L: SharingPolicy,
    R: RetryPolicy,
{
    inner: UnsafeCell<T>,
    lock:  L,
    retry: R,
}

// SAFETY: The lock ensures that concurrent access to `T` is properly
// synchronized. As long as `T` is `Send + Sync`, the lock itself can be safely
// shared across threads.
unsafe impl<T, L, R> core::marker::Sync for Sharex<T, L, R>
where
    T: Send + Sync,
    L: SharingPolicy,
    R: RetryPolicy,
{
}

// SAFETY: The lock can be safely moved between threads as long as `T` is
// `Send`.
unsafe impl<T, L, R> core::marker::Send for Sharex<T, L, R>
where
    T: Send,
    L: SharingPolicy,
    R: RetryPolicy,
{
}

impl<T, L, R> core::default::Default for Sharex<T, L, R>
where
    T: Default,
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
{
    fn default() -> Self
    {
        Self {
            inner: UnsafeCell::default(),
            lock:  L::default(),
            retry: R::default(),
        }
    }
}

impl<T, L, R> Sharex<T, L, R>
where
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
{
    /// Creates a new `Sharex` lock protecting the given `value`.
    ///
    /// The lock and retry policies are initialized using their `Default`
    /// implementations.
    pub fn new(value: T) -> Self
    {
        Self {
            inner: UnsafeCell::new(value),
            lock:  L::default(),
            retry: R::default(),
        }
    }

    /// Attempts to acquire a shared (read) lock without blocking.
    ///
    /// This method calls [`SharingPolicy::try_share`] exactly once. If the lock
    /// is currently held exclusively by a writer, it returns
    /// `Err(TryLockError::Contention)`.
    ///
    /// # Returns
    ///
    /// - `Ok(guard)`: The shared lock was successfully acquired.
    /// - `Err(TryLockError::Contention)`: The lock is currently held
    ///   exclusively.
    /// - `Err(TryLockError::Lock(e))`: An unrecoverable error occurred.
    pub fn try_read(&self)
    -> Result<ShGuard<'_, T, L>, TryLockError<L::Error>>
    {
        match self.lock.try_share(0)
        {
            Ok(LockStatus::Done(meta)) =>
            {
                Ok(ShGuard::new(self.inner.get(), &self.lock, meta))
            },
            Ok(LockStatus::Fail) => Err(TryLockError::Contention),
            Err(e) => Err(TryLockError::Lock(e)),
        }
    }

    /// Attempts to acquire an exclusive (write) lock without blocking.
    ///
    /// This method calls [`crate::traits::LockPolicy::try_lock`] exactly once.
    /// If the lock is currently held by any reader or writer, it returns
    /// `Err(TryLockError::Contention)`.
    ///
    /// # Returns
    ///
    /// - `Ok(guard)`: The exclusive lock was successfully acquired.
    /// - `Err(TryLockError::Contention)`: The lock is currently held.
    /// - `Err(TryLockError::Lock(e))`: An unrecoverable error occurred.
    pub fn try_write(&self)
    -> Result<ExGuard<'_, T, L>, TryLockError<L::Error>>
    {
        match unsafe { self.lock.try_lock(0) }
        {
            Ok(LockStatus::Done(meta)) =>
            {
                Ok(ExGuard::new(self.inner.get(), &self.lock, meta))
            },
            Ok(LockStatus::Fail) => Err(TryLockError::Contention),
            Err(e) => Err(TryLockError::Lock(e)),
        }
    }

    /// Acquires a shared (read) lock, blocking the current thread until it is
    /// available.
    ///
    /// This method repeatedly calls [`SharingPolicy::try_share`]. If the lock
    /// is not immediately available, it calls [`RetryPolicy::retry`] to wait
    /// (e.g., by spinning or yielding) before trying again.
    ///
    /// # Returns
    ///
    /// - `Ok(guard)`: The shared lock was successfully acquired.
    /// - `Err(LockError::Lock(e))`: An unrecoverable error occurred in the lock
    ///   policy.
    /// - `Err(LockError::Retry(e))`: The retry policy aborted the acquisition
    ///   loop.
    pub fn read(
        &self,
    ) -> Result<ShGuard<'_, T, L>, LockError<L::Error, <R as RetryPolicy>::Error>>
    {
        let mut iterations = 0usize;

        loop
        {
            iterations += 1;

            match self.lock.try_share(iterations)
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    return Ok(ShGuard::new(self.inner.get(), &self.lock, meta));
                },
                Ok(LockStatus::Fail) =>
                {
                    if let Err(e) = self.retry.retry(iterations)
                    {
                        return Err(LockError::Retry(e));
                    }
                },
                Err(e) => return Err(LockError::Lock(e)),
            }
        }
    }

    /// Acquires an exclusive (write) lock, blocking the current thread until it
    /// is available.
    ///
    /// This method repeatedly calls [`crate::traits::LockPolicy::try_lock`]. If
    /// the lock is not immediately available, it calls
    /// [`RetryPolicy::retry`] to wait (e.g., by spinning or yielding)
    /// before trying again.
    ///
    /// # Returns
    ///
    /// - `Ok(guard)`: The exclusive lock was successfully acquired.
    /// - `Err(LockError::Lock(e))`: An unrecoverable error occurred in the lock
    ///   policy.
    /// - `Err(LockError::Retry(e))`: The retry policy aborted the acquisition
    ///   loop.
    pub fn write(
        &self,
    ) -> Result<ExGuard<'_, T, L>, LockError<L::Error, <R as RetryPolicy>::Error>>
    {
        let mut iterations = 0usize;

        loop
        {
            iterations += 1;

            match unsafe { self.lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    return Ok(ExGuard::new(self.inner.get(), &self.lock, meta));
                },
                Ok(LockStatus::Fail) =>
                {
                    if let Err(e) = self.retry.retry(iterations)
                    {
                        return Err(LockError::Retry(e));
                    }
                },
                Err(e) => return Err(LockError::Lock(e)),
            }
        }
    }

    /// Exchanges the protected value with `new_value`, returning the old value.
    ///
    /// This acquires an exclusive write lock before performing the exchange.
    ///
    /// # Returns
    ///
    /// - `Ok(old_value)`: Exchange succeeded.
    /// - `Err(LockError::Lock(e))`: Unrecoverable lock error.
    /// - `Err(LockError::Retry(e))`: Retry policy aborted.
    pub fn exchange(
        &self,
        new_value: T,
    ) -> Result<T, LockError<L::Error, <R as RetryPolicy>::Error>>
    {
        let guard = self.write()?;
        Ok(guard.exchange(new_value))
    }

    /// Non‑blocking version of [`exchange`](Self::exchange).
    ///
    /// Attempts to acquire a write lock without waiting. If successful,
    /// exchanges the protected value with `new_value`, returns the old
    /// value, and releases the lock.
    ///
    /// # Returns
    ///
    /// - `Ok(old_value)`: The lock was immediately available and the exchange
    ///   succeeded.
    /// - `Err(TryLockError::Contention)`: The lock is currently held (by any
    ///   writer or reader).
    /// - `Err(TryLockError::Lock(e))`: An unrecoverable error occurred in the
    ///   lock policy.
    pub fn try_exchange(
        &self,
        new_value: T,
    ) -> Result<T, TryLockError<L::Error>>
    {
        Ok(self.try_write()?.exchange(new_value))
    }
}

impl<T, L, R> Sharex<T, L, R>
where
    T: Default,
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
{
    /// Takes the value out of the mutex, leaving a `Default::default()` value
    /// in its place.
    ///
    /// This is equivalent to acquiring the lock and calling [`core::mem::take`]
    /// on the protected data.
    ///
    /// # Returns
    ///
    /// - `Ok(value)`: The lock was successfully acquired and the value was
    ///   taken.
    /// - `Err(LockError::Lock(e))`: An unrecoverable error occurred in the lock
    ///   policy.
    /// - `Err(LockError::Retry(e))`: The retry policy aborted the acquisition
    ///   loop.
    pub fn take(
        &self,
    ) -> Result<T, LockError<L::Error, <R as RetryPolicy>::Error>>
    {
        Ok(self.write()?.take())
    }

    /// Non‑blocking version of a `take` operation.
    ///
    /// Attempts to acquire a write lock without waiting. If successful, takes
    /// the protected value, replaces it with `Default::default()`, and
    /// releases the lock.
    ///
    /// # Returns
    ///
    /// - `Ok(value)`: The lock was immediately available and the value was
    ///   taken.
    /// - `Err(TryLockError::Contention)`: The lock is currently held (by any
    ///   writer or reader).
    /// - `Err(TryLockError::Lock(e))`: An unrecoverable error occurred in the
    ///   lock policy.
    pub fn try_take(&self) -> Result<T, TryLockError<L::Error>>
    {
        Ok(self.try_write()?.take())
    }
}
