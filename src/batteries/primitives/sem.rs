//! A counting semaphore primitive.
//!
//! This module provides the [`Semaphore`] struct, which maintains a set of
//! permits. Permits are acquired and released by threads to synchronize access
//! to a pool of resources or to limit concurrency.
//!
//! # Design
//!
//! Unlike standard library semaphores, this implementation is fully composable
//! and generic over the underlying [`LockPolicy`] and [`RetryPolicy`]. It uses
//! the lock policy to protect the internal permit counter and relies on the
//! lock's native parking/waking mechanisms (e.g., futexes in `lock::Os`) when
//! contention occurs. This makes it fully compatible with `#![no_std]`
//! environments when paired with `lock::Atomic` and `retry::Busy`.

use crate::traits::{LockPolicy, RetryPolicy};
use crate::{AcquireError, LockStatus, TryLockError};
use core::cell::UnsafeCell;

/// A counting semaphore that limits concurrent access to a pool of resources.
///
/// A semaphore holds a certain number of "permits". Threads can acquire permits
/// (blocking if none are available) and release them when done.
///
/// # Type Parameters
///
/// - `L`: The [`LockPolicy`] used to protect the internal permit counter.
/// - `R`: The [`RetryPolicy`] used to wait when the lock protecting the counter
///   is contended or when no permits are available.
#[allow(missing_debug_implementations)]
pub struct Semaphore<L = crate::lock::Os, R = crate::retry::Yield>
where
    L: LockPolicy,
    R: RetryPolicy,
{
    count: UnsafeCell<usize>,
    lock:  L,
    retry: R,
}

// SAFETY:
// The semaphore ensures exclusive access to `count` via the lock policy.
unsafe impl<L, R> core::marker::Sync for Semaphore<L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
}

// SAFETY:
// The semaphore can be safely moved between threads as long as its policies
// are Send.
unsafe impl<L, R> core::marker::Send for Semaphore<L, R>
where
    L: LockPolicy + core::marker::Send,
    R: RetryPolicy + core::marker::Send,
{
}

impl<L, R> Semaphore<L, R>
where
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
{
    /// Creates a new semaphore with the specified number of initial permits.
    pub fn new(permits: usize) -> Self
    {
        Self {
            count: UnsafeCell::new(permits),
            lock:  L::default(),
            retry: R::default(),
        }
    }
}

impl<L, R> core::default::Default for Semaphore<L, R>
where
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
{
    /// Creates a new semaphore with `0` initial permits.
    fn default() -> Self
    {
        Self::new(0)
    }
}

impl<L, R> From<(usize, L, R)> for Semaphore<L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
    fn from(value: (usize, L, R)) -> Self
    {
        Self {
            count: UnsafeCell::new(value.0),
            lock:  value.1,
            retry: value.2,
        }
    }
}

impl<L, R> Semaphore<L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
    /// Acquires a single permit from the semaphore.
    ///
    /// This method blocks the current thread until a permit is available.
    pub fn acquire(&self) -> Result<(), AcquireError<(), L::Error, R::Error>>
    {
        self.acquire_many(1)
    }

    /// Acquires `n` permits from the semaphore.
    ///
    /// This method blocks the current thread until `n` permits are available.
    /// This is useful for weighted resource allocation.
    pub fn acquire_many(
        &self,
        n: usize,
    ) -> Result<(), AcquireError<(), L::Error, R::Error>>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;
            match unsafe { self.lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    let count = unsafe { &mut *self.count.get() };
                    if *count >= n
                    {
                        *count -= n;
                        unsafe { self.lock.free(&meta) };
                        return Ok(());
                    }
                    else
                    {
                        // Not enough permits, release lock and wait
                        unsafe { self.lock.free(&meta) };
                        if let Err(e) = self.retry.retry(iterations)
                        {
                            return Err(AcquireError::Retry(e));
                        }
                    }
                },
                Ok(LockStatus::Fail) =>
                {
                    if let Err(e) = self.retry.retry(iterations)
                    {
                        return Err(AcquireError::Retry(e));
                    }
                },
                Err(e) => return Err(AcquireError::Lock(e)),
            }
        }
    }

    /// Attempts to acquire a single permit without blocking.
    pub fn try_acquire(&self) -> Result<(), TryLockError<(), L::Error>>
    {
        self.try_acquire_many(1)
    }

    /// Attempts to acquire `n` permits without blocking.
    pub fn try_acquire_many(
        &self,
        n: usize,
    ) -> Result<(), TryLockError<(), L::Error>>
    {
        match unsafe { self.lock.try_lock(0) }
        {
            Ok(LockStatus::Done(meta)) =>
            {
                let count = unsafe { &mut *self.count.get() };
                if *count >= n
                {
                    *count -= n;
                    unsafe { self.lock.free(&meta) };
                    Ok(())
                }
                else
                {
                    unsafe { self.lock.free(&meta) };
                    Err(TryLockError::Contention)
                }
            },
            Ok(LockStatus::Fail) => Err(TryLockError::Contention),
            Err(e) => Err(TryLockError::Lock(e)),
        }
    }

    /// Releases a single permit back to the semaphore.
    pub fn release(&self) -> Result<(), AcquireError<(), L::Error, R::Error>>
    {
        self.release_many(1)
    }

    /// Releases `n` permits back to the semaphore.
    pub fn release_many(
        &self,
        n: usize,
    ) -> Result<(), AcquireError<(), L::Error, R::Error>>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;
            match unsafe { self.lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    let count = unsafe { &mut *self.count.get() };
                    *count += n;
                    unsafe { self.lock.free(&meta) };

                    // Wake up threads that might be waiting on the lock itself
                    // (e.g., inside a futex-based LockPolicy like `lock::Os`).
                    self.lock.wake_all();
                    return Ok(());
                },
                Ok(LockStatus::Fail) =>
                {
                    if let Err(e) = self.retry.retry(iterations)
                    {
                        return Err(AcquireError::Retry(e));
                    }
                },
                Err(e) => return Err(AcquireError::Lock(e)),
            }
        }
    }

    /// Returns the number of currently available permits.
    ///
    /// This method acquires the underlying lock to read the count safely,
    /// so it may block if the lock is highly contended.
    pub fn available_permits(
        &self,
    ) -> Result<usize, AcquireError<(), L::Error, R::Error>>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;
            match unsafe { self.lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    let count = unsafe { *self.count.get() };
                    unsafe { self.lock.free(&meta) };
                    return Ok(count);
                },
                Ok(LockStatus::Fail) =>
                {
                    if let Err(e) = self.retry.retry(iterations)
                    {
                        return Err(AcquireError::Retry(e));
                    }
                },
                Err(e) => return Err(AcquireError::Lock(e)),
            }
        }
    }
}
