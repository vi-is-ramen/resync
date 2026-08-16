//! A shareable-exclusive (read-write) lock primitive.

use crate::traits::{RetryPolicy, SharingPolicy};
use crate::{
    AcquireError, ExGuard, LockStatus, PoisonError, ShGuard, TryLockError,
};
use core::cell::UnsafeCell;
#[cfg(feature = "std")]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "__lint")]
use crate::lock::Atomic as DefaultLock;

#[cfg(not(feature = "__lint"))]
use crate::lock::Os as DefaultLock;

/// A shareable-exclusive (read-write) lock primitive that protects a value of
/// type `T`.
#[allow(missing_debug_implementations)]
pub struct Sharex<
    T,
    L = crate::lock::Shield<DefaultLock>,
    R = crate::retry::Yield,
> where
    L: SharingPolicy,
    R: RetryPolicy,
{
    inner:    UnsafeCell<T>,
    lock:     L,
    retry:    R,
    #[cfg(feature = "std")]
    poisoned: AtomicBool,
}

unsafe impl<T, L, R> core::marker::Sync for Sharex<T, L, R>
where
    L: SharingPolicy,
    R: RetryPolicy,
{
}

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
            inner:                            UnsafeCell::default(),
            lock:                             L::default(),
            retry:                            R::default(),
            #[cfg(feature = "std")]
            poisoned:                         AtomicBool::new(false),
        }
    }
}

/// Result type for non-blocking [`Sharex::try_read`] operations.
///
/// # Errors
///
/// Returns a [`TryLockError`] if the lock is currently held exclusively
/// by a writer (`Contention`), if an unrecoverable error occurs in the
/// underlying sharing policy (`Lock`), or if the lock was poisoned
/// (`Poisoned`).
pub type SharexTryReadResult<'a, T, L>
where L: SharingPolicy + Default
= Result<ShGuard<'a, T, L>, TryLockError<ShGuard<'a, T, L>, L::Error>>;

/// Result type for non-blocking [`Sharex::try_write`] operations.
///
/// # Errors
///
/// Returns a [`TryLockError`] if the lock is currently held by any reader
/// or writer (`Contention`), if an unrecoverable error occurs, or if the
/// lock was poisoned.
pub type SharexTryWriteResult<'a, T, L>
where L: SharingPolicy + Default
= Result<ExGuard<'a, T, L>, TryLockError<ExGuard<'a, T, L>, L::Error>>;

/// Result type for blocking [`Sharex::read`] operations.
///
/// # Errors
///
/// Returns an [`AcquireError`] if the lock was poisoned, if an unrecoverable
/// error occurs in the underlying sharing policy, or if the retry policy
/// aborts the acquisition loop.
pub type SharexReadResult<'a, T, L, R>
where
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
= Result<
    ShGuard<'a, T, L>,
    AcquireError<ShGuard<'a, T, L>, L::Error, <R as RetryPolicy>::Error>,
>;

/// Result type for blocking [`Sharex::write`] operations.
///
/// # Errors
///
/// Returns an [`AcquireError`] if the lock was poisoned, if an unrecoverable
/// error occurs, or if the retry policy aborts.
pub type SharexWriteResult<'a, T, L, R>
where
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
= Result<
    ExGuard<'a, T, L>,
    AcquireError<ExGuard<'a, T, L>, L::Error, <R as RetryPolicy>::Error>,
>;

/// Result type for blocking [`Sharex::exchange`] and [`Sharex::take`]
/// operations.
///
/// # Errors
///
/// Returns an [`AcquireError`] if the lock was poisoned, if an unrecoverable
/// lock error occurs, or if the retry policy aborts.
pub type SharexExchangeResult<'a, T, L, R>
where
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
= Result<
    T,
    AcquireError<ExGuard<'a, T, L>, L::Error, <R as RetryPolicy>::Error>,
>;

impl<T, L, R> Sharex<T, L, R>
where
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
{
    /// Creates a new `Sharex` lock protecting the given `value`.
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

impl<T, L, R> Sharex<T, L, R>
where
    L: SharingPolicy,
    R: RetryPolicy,
{
    /// Attempts to acquire a shared (read) lock without blocking.
    pub fn try_read(&self) -> SharexTryReadResult<'_, T, L>
    {
        match self.lock.try_share(0)
        {
            Ok(LockStatus::Done(meta)) =>
            {
                #[cfg(feature = "std")]
                let is_poisoned = self.poisoned.load(Ordering::Acquire);
                #[cfg(not(feature = "std"))]
                let is_poisoned = false;

                #[cfg(feature = "std")]
                let guard = ShGuard::new(
                    self.inner.get(),
                    &self.lock,
                    meta,
                    Some(&self.poisoned),
                );
                #[cfg(not(feature = "std"))]
                let guard = ShGuard::new(self.inner.get(), &self.lock, meta);

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

    /// Attempts to acquire an exclusive (write) lock without blocking.
    pub fn try_write(&self) -> SharexTryWriteResult<'_, T, L>
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

    /// Acquires a shared (read) lock, blocking the current thread until it is
    /// available.
    pub fn read(&self) -> SharexReadResult<'_, T, L, R>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;
            match self.lock.try_share(iterations)
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    #[cfg(feature = "std")]
                    let is_poisoned = self.poisoned.load(Ordering::Acquire);
                    #[cfg(not(feature = "std"))]
                    let is_poisoned = false;

                    #[cfg(feature = "std")]
                    let guard = ShGuard::new(
                        self.inner.get(),
                        &self.lock,
                        meta,
                        Some(&self.poisoned),
                    );
                    #[cfg(not(feature = "std"))]
                    let guard =
                        ShGuard::new(self.inner.get(), &self.lock, meta);

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

    /// Acquires an exclusive (write) lock, blocking the current thread until it
    /// is available.
    pub fn write(&self) -> SharexWriteResult<'_, T, L, R>
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
    pub fn exchange(&self, new_value: T) -> SharexExchangeResult<'_, T, L, R>
    {
        let guard = self.write()?;
        Ok(guard.exchange(new_value))
    }

    /// Non‑blocking version of [`exchange`](Self::exchange).
    pub fn try_exchange(
        &self,
        new_value: T,
    ) -> Result<T, TryLockError<ExGuard<'_, T, L>, L::Error>>
    {
        Ok(self.try_write()?.exchange(new_value))
    }

    /// Returns `true` if the lock is poisoned.
    #[cfg(feature = "std")]
    pub fn is_poisoned(&self) -> bool
    {
        self.poisoned.load(Ordering::Acquire)
    }

    /// Clears the poisoned state of the lock.
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

impl<T, L, R> Sharex<T, L, R>
where
    T: Default,
    L: SharingPolicy,
    R: RetryPolicy,
{
    /// Takes the value out of the lock, leaving a `Default::default()` value in
    /// its place.
    pub fn take(&self) -> SharexExchangeResult<'_, T, L, R>
    {
        Ok(self.write()?.take())
    }

    /// Non‑blocking version of a `take` operation.
    pub fn try_take(
        &self,
    ) -> Result<T, TryLockError<ExGuard<'_, T, L>, L::Error>>
    {
        Ok(self.try_write()?.take())
    }
}

impl<'a, T, L, R>
    crate::api::Mutex<
        'a,
        T,
        SharexTryWriteResult<'a, T, L>,
        SharexWriteResult<'a, T, L, R>,
    > for Sharex<T, L, R>
where
    L: SharingPolicy,
    R: RetryPolicy,
{
    fn lock(&'a self) -> SharexWriteResult<'a, T, L, R>
    {
        self.write()
    }

    fn try_lock(&'a self) -> SharexTryWriteResult<'a, T, L>
    {
        self.try_write()
    }
}

impl<'a, T, L, R>
    crate::api::Sharex<
        'a,
        T,
        SharexTryReadResult<'a, T, L>,
        SharexReadResult<'a, T, L, R>,
    > for Sharex<T, L, R>
where
    L: SharingPolicy,
    R: RetryPolicy,
{
    fn try_read(&'a self) -> SharexTryReadResult<'a, T, L>
    {
        self.try_read()
    }

    fn read(&'a self) -> SharexReadResult<'a, T, L, R>
    {
        self.read()
    }
}
