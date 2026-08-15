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

use crate::LockStatus;
use crate::traits::{RetryPolicy, SharingPolicy};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// Errors that can occur when acquiring a [`Sharex`] lock via `read` or
/// `write`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharexError<LE, RE>
{
    /// An unrecoverable error occurred in the underlying lock policy.
    Lock(LE),
    /// The retry policy aborted the acquisition loop (e.g., due to a timeout).
    Retry(RE),
}

/// Error returned by `try_read` and `try_write` when the lock is contended or
/// fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryLockError<LE>
{
    /// The lock is currently held by another thread.
    Contention,
    /// An unrecoverable error occurred in the underlying lock policy.
    Lock(LE),
}

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
pub struct Sharex<T, L = crate::lock::Os, R = crate::retry::Yield>
where
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

impl<T: core::default::Default, L, R> core::default::Default for Sharex<T, L, R>
where
    T: Default,
    L: SharingPolicy,
    R: RetryPolicy,
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

/// A RAII guard that provides shared (read) access to the protected data.
///
/// When this guard is dropped, the shared lock is automatically released
/// via the [`SharingPolicy::free_share`] method.
#[allow(missing_debug_implementations)]
pub struct ReadGuard<'a, T, L: SharingPolicy>
{
    data: *const T,
    lock: &'a L,
}

impl<'a, T, L: SharingPolicy> core::ops::Drop for ReadGuard<'a, T, L>
{
    /// Releases the shared lock held by this guard.
    fn drop(&mut self)
    {
        self.lock.free_share();
    }
}

impl<'a, T, L: SharingPolicy> Deref for ReadGuard<'a, T, L>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        // SAFETY: The guard guarantees shared access to the data, and no
        // mutable references can exist while this guard is alive.
        unsafe { self.data.as_ref_unchecked() }
    }
}

/// A RAII guard that provides exclusive (write) access to the protected data.
///
/// When this guard is dropped, the exclusive lock is automatically released
/// via the [`crate::traits::LockPolicy::free`] method.
#[allow(missing_debug_implementations)]
pub struct WriteGuard<'a, T, L: SharingPolicy>
{
    data: *mut T,
    lock: &'a L,
}

impl<'a, T, L: SharingPolicy> core::ops::Drop for WriteGuard<'a, T, L>
{
    /// Releases the exclusive lock held by this guard.
    ///
    /// # Safety
    ///
    /// This calls the unsafe `free` method on the underlying lock policy.
    /// It is safe here because the guard's existence guarantees that the
    /// exclusive lock is currently held by the current thread.
    fn drop(&mut self)
    {
        unsafe { self.lock.free() };
    }
}

impl<'a, T, L: SharingPolicy> Deref for WriteGuard<'a, T, L>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        // SAFETY: The guard guarantees exclusive access to the data.
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L: SharingPolicy> DerefMut for WriteGuard<'a, T, L>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        // SAFETY: The guard guarantees exclusive access to the data.
        unsafe { self.data.as_mut_unchecked() }
    }
}

impl<T, L: SharingPolicy, R: RetryPolicy> Sharex<T, L, R>
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
    pub fn try_read(
        &self,
    ) -> Result<ReadGuard<'_, T, L>, TryLockError<L::Error>>
    {
        match self.lock.try_share(0)
        {
            Ok(LockStatus::Done) => Ok(ReadGuard {
                data: self.inner.get(),
                lock: &self.lock,
            }),
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
    pub fn try_write(
        &self,
    ) -> Result<WriteGuard<'_, T, L>, TryLockError<L::Error>>
    {
        match unsafe { self.lock.try_lock(0) }
        {
            Ok(LockStatus::Done) => Ok(WriteGuard {
                data: self.inner.get(),
                lock: &self.lock,
            }),
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
    /// - `Err(SharexError::Lock(e))`: An unrecoverable error occurred in the
    ///   lock policy.
    /// - `Err(SharexError::Retry(e))`: The retry policy aborted the acquisition
    ///   loop.
    pub fn read(
        &self,
    ) -> Result<ReadGuard<'_, T, L>, SharexError<L::Error, R::Error>>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;

            match self.lock.try_share(iterations)
            {
                Ok(LockStatus::Done) =>
                {
                    return Ok(ReadGuard {
                        data: self.inner.get(),
                        lock: &self.lock,
                    });
                },
                Ok(LockStatus::Fail) =>
                {
                    if let Err(e) = self.retry.retry(iterations)
                    {
                        return Err(SharexError::Retry(e));
                    }
                },
                Err(e) => return Err(SharexError::Lock(e)),
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
    /// - `Err(SharexError::Lock(e))`: An unrecoverable error occurred in the
    ///   lock policy.
    /// - `Err(SharexError::Retry(e))`: The retry policy aborted the acquisition
    ///   loop.
    pub fn write(
        &self,
    ) -> Result<WriteGuard<'_, T, L>, SharexError<L::Error, R::Error>>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;

            match unsafe { self.lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done) =>
                {
                    return Ok(WriteGuard {
                        data: self.inner.get(),
                        lock: &self.lock,
                    });
                },
                Ok(LockStatus::Fail) =>
                {
                    if let Err(e) = self.retry.retry(iterations)
                    {
                        return Err(SharexError::Retry(e));
                    }
                },
                Err(e) => return Err(SharexError::Lock(e)),
            }
        }
    }
}
