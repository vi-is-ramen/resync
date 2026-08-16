//! A mutual exclusion primitive that composes a lock policy and a retry policy.

use super::ExGuard;
use crate::traits::{LockPolicy, RetryPolicy};
use crate::{AcquireError, LockStatus, PoisonError, TryLockError};
use core::cell::UnsafeCell;
#[cfg(feature = "std")]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "__lint")]
use crate::lock::Atomic as DefaultLock;

#[cfg(not(feature = "__lint"))]
use crate::lock::Os as DefaultLock;

/// A mutual exclusion (mutex) primitive that protects a value of type `T`.
#[allow(missing_debug_implementations)]
pub struct Mutex<T, L = DefaultLock, R = crate::retry::Yield>
where
    L: LockPolicy,
    R: RetryPolicy,
{
    inner:    UnsafeCell<T>,
    lock:     L,
    retry:    R,
    #[cfg(feature = "std")]
    poisoned: AtomicBool,
}

impl<T, L, R> core::fmt::Debug for Mutex<T, L, R>
where
    T: core::fmt::Debug,
    L: LockPolicy,
    R: RetryPolicy,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.write_str("Mutex { ")?;
        <T as core::fmt::Debug>::fmt(
            unsafe { self.inner.get().as_ref_unchecked() },
            f,
        )?;
        f.write_str(" }")?;
        Ok(())
    }
}

unsafe impl<T, L, R> core::marker::Sync for Mutex<T, L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
}

unsafe impl<T, L, R> core::marker::Send for Mutex<T, L, R>
where
    T: core::marker::Send,
    L: LockPolicy + core::marker::Send,
    R: RetryPolicy + core::marker::Send,
{
}

impl<T, L, R> core::default::Default for Mutex<T, L, R>
where
    T: Default,
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
{
    fn default() -> Self
    {
        Self {
            inner:                            UnsafeCell::new(T::default()),
            lock:                             L::default(),
            retry:                            R::default(),
            #[cfg(feature = "std")]
            poisoned:                         AtomicBool::new(false),
        }
    }
}

/// Result type for non-blocking [`Mutex::try_lock`] operations.
///
/// # Errors
///
/// Returns a [`TryLockError`] if the lock is currently held by another
/// thread (`Contention`), if an unrecoverable error occurs in the
/// underlying lock policy (`Lock`), or if the lock was poisoned by a
/// panicking thread (`Poisoned`).
pub type MutexTryLockResult<'a, T, L>
where L: LockPolicy + Default
= Result<
    ExGuard<'a, T, L, L::Meta>,
    TryLockError<ExGuard<'a, T, L, L::Meta>, L::Error>,
>;

/// Result type for blocking [`Mutex::lock`] operations.
///
/// # Errors
///
/// Returns an [`AcquireError`] if the lock was poisoned by a panicking
/// thread, if an unrecoverable error occurs in the underlying lock policy,
/// or if the retry policy aborts the acquisition loop (e.g., due to a
/// timeout).
pub type MutexLockResult<'a, T, L, R>
where
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
= Result<
    ExGuard<'a, T, L>,
    AcquireError<ExGuard<'a, T, L>, L::Error, <R as RetryPolicy>::Error>,
>;

/// Result type for blocking [`Mutex::exchange`] and [`Mutex::take`] operations.
///
/// # Errors
///
/// Returns an [`AcquireError`] if the lock was poisoned, if an unrecoverable
/// lock error occurs, or if the retry policy aborts.
pub type MutexExchangeResult<'a, T, L, R>
where
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
= Result<
    T,
    AcquireError<ExGuard<'a, T, L>, L::Error, <R as RetryPolicy>::Error>,
>;

/// Result type for non-blocking [`Mutex::try_exchange`] and [`Mutex::try_take`]
/// operations.
///
/// # Errors
///
/// Returns a [`TryLockError`] if the lock is currently held, if an
/// unrecoverable lock error occurs, or if the lock is poisoned.
pub type MutexTryExchangeResult<'a, T, L>
where L: LockPolicy + Default
= Result<T, TryLockError<ExGuard<'a, T, L, L::Meta>, L::Error>>;

impl<T, L, R> Mutex<T, L, R>
where
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
{
    /// Creates a new mutex protecting the given `value`.
    pub fn new(value: T) -> Self
    {
        Self {
            inner:                            UnsafeCell::new(value),
            lock:                             L::default(),
            retry:                            R::default(),
            #[cfg(feature = "std")]
            poisoned:                         AtomicBool::new(false),
        }
    }
}

impl<T, L, R> Mutex<T, L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
    /// Attempts to acquire the mutex without blocking.
    pub fn try_lock(&self) -> MutexTryLockResult<'_, T, L>
    {
        match unsafe { self.lock.try_lock(0) }
        {
            Ok(LockStatus::Done(meta)) =>
            {
                #[cfg(feature = "std")]
                let is_poisoned = self.poisoned.load(Ordering::Acquire);
                #[cfg(not(feature = "std"))]
                let is_poisoned = false;

                #[cfg(feature = "std")]
                let guard = ExGuard::new(
                    self.inner.get(),
                    &self.lock,
                    meta,
                    Some(&self.poisoned),
                );
                #[cfg(not(feature = "std"))]
                let guard = ExGuard::new(self.inner.get(), &self.lock, meta);

                if is_poisoned
                {
                    Err(TryLockError::Poisoned(PoisonError::new(guard)))
                }
                else
                {
                    Ok(guard)
                }
            },
            Ok(LockStatus::Fail) => Err(TryLockError::Contention),
            Err(e) => Err(TryLockError::Lock(e)),
        }
    }

    /// Acquires the mutex, blocking the current thread until it is available.
    pub fn lock(&self) -> MutexLockResult<'_, T, L, R>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;
            match unsafe { self.lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    #[cfg(feature = "std")]
                    let is_poisoned = self.poisoned.load(Ordering::Acquire);
                    #[cfg(not(feature = "std"))]
                    let is_poisoned = false;

                    #[cfg(feature = "std")]
                    let guard = ExGuard::new(
                        self.inner.get(),
                        &self.lock,
                        meta,
                        Some(&self.poisoned),
                    );
                    #[cfg(not(feature = "std"))]
                    let guard =
                        ExGuard::new(self.inner.get(), &self.lock, meta);

                    if is_poisoned
                    {
                        return Err(AcquireError::Poisoned(PoisonError::new(
                            guard,
                        )));
                    }
                    return Ok(guard);
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

    /// Exchanges the protected value with `new_value`, returning the old value.
    pub fn exchange(&self, new_value: T) -> MutexExchangeResult<'_, T, L, R>
    {
        let guard = self.lock()?;
        Ok(guard.exchange(new_value))
    }

    /// Non‑blocking version of [`exchange`](Self::exchange).
    pub fn try_exchange(&self, new_value: T)
    -> MutexTryExchangeResult<'_, T, L>
    {
        Ok(self.try_lock()?.exchange(new_value))
    }

    /// Returns `true` if the mutex is poisoned.
    #[cfg(feature = "std")]
    pub fn is_poisoned(&self) -> bool
    {
        self.poisoned.load(Ordering::Acquire)
    }

    /// Clears the poisoned state of the mutex.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the protected data has been manually
    /// repaired or validated before calling this method.
    #[cfg(feature = "std")]
    pub unsafe fn clear_poison(&self)
    {
        self.poisoned.store(false, Ordering::Release);
    }
}

impl<T, L, R> Mutex<T, L, R>
where
    T: Default,
    L: LockPolicy,
    R: RetryPolicy,
{
    /// Takes the value out of the mutex, leaving a `Default::default()` value
    /// in its place.
    pub fn take(&self) -> MutexExchangeResult<'_, T, L, R>
    {
        Ok(self.lock()?.take())
    }

    /// Non‑blocking version of [`take`](Self::take).
    pub fn try_take(&self) -> MutexTryExchangeResult<'_, T, L>
    {
        Ok(self.try_lock()?.take())
    }
}

impl<'a, T, L, R>
    crate::api::Mutex<
        'a,
        T,
        MutexTryLockResult<'a, T, L>,
        MutexLockResult<'a, T, L, R>,
    > for Mutex<T, L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
    fn lock(&'a self) -> MutexLockResult<'a, T, L, R>
    {
        self.lock()
    }

    fn try_lock(&'a self) -> MutexTryLockResult<'a, T, L>
    {
        self.try_lock()
    }
}
