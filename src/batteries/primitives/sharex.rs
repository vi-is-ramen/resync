//! A shareable-exclusive (read-write) lock primitive.
#[cfg(dev)]
use crate::api::ForceUnlock;
use crate::api::{PoisonPolicy, RetryPolicy, SharingPolicy};
use crate::{
    AcquireError, ExGuard, LockStatus, PoisonError, ShGuard, TryLockError,
};
use core::cell::UnsafeCell;

/// A shareable-exclusive (read-write) lock primitive that protects a value of
/// type `T`.
#[allow(missing_debug_implementations)]
pub struct Sharex<
    T,
    L = crate::lock::Shield<crate::lock::DefaultLock>,
    R = crate::retry::DefaultRetry,
    P = crate::poison::DefaultPoison,
> where
    L: SharingPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    inner:    UnsafeCell<T>,
    lock:     L,
    retry:    R,
    poisoned: P,
}

unsafe impl<T, L, R, P> core::marker::Sync for Sharex<T, L, R, P>
where
    L: SharingPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
}

unsafe impl<T, L, R, P> core::marker::Send for Sharex<T, L, R, P>
where
    T: Send,
    L: SharingPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
}

impl<T, L, R, P> core::default::Default for Sharex<T, L, R, P>
where
    T: Default,
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy + Default,
{
    fn default() -> Self
    {
        Self {
            inner:    UnsafeCell::default(),
            lock:     L::default(),
            retry:    R::default(),
            poisoned: P::default(),
        }
    }
}

/// Result type for non-blocking [`Sharex::try_read`] operations.
pub type SharexTryReadResult<'a, T, L, P>
where
    L: SharingPolicy,
    P: PoisonPolicy,
= Result<ShGuard<'a, T, L, P>, TryLockError<ShGuard<'a, T, L, P>, L::Error>>;

/// Result type for non-blocking [`Sharex::try_write`] operations.
pub type SharexTryWriteResult<'a, T, L, P>
where
    L: SharingPolicy,
    P: PoisonPolicy,
= Result<ExGuard<'a, T, L, P>, TryLockError<ExGuard<'a, T, L, P>, L::Error>>;

/// Result type for blocking [`Sharex::read`] operations.
pub type SharexReadResult<'a, T, L, R, P>
where
    L: SharingPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
= Result<
    ShGuard<'a, T, L, P>,
    AcquireError<ShGuard<'a, T, L, P>, L::Error, <R as RetryPolicy>::Error>,
>;

/// Result type for blocking [`Sharex::write`] operations.
pub type SharexWriteResult<'a, T, L, R, P>
where
    L: SharingPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
= Result<
    ExGuard<'a, T, L, P>,
    AcquireError<ExGuard<'a, T, L, P>, L::Error, <R as RetryPolicy>::Error>,
>;

/// Result type for blocking [`Sharex::exchange`] and [`Sharex::take`]
/// operations.
pub type SharexExchangeResult<'a, T, L, R, P>
where
    L: SharingPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
= Result<
    T,
    AcquireError<ExGuard<'a, T, L, P>, L::Error, <R as RetryPolicy>::Error>,
>;

/// Result type for blocking [`Sharex::try_exchange`] and [`Sharex::try_take`]
/// operations.
pub type SharexTryExchangeResult<'a, T, L, P>
where
    L: SharingPolicy,
    P: PoisonPolicy,
= Result<T, TryLockError<ExGuard<'a, T, L, P>, L::Error>>;

impl<T, L, R, P> Sharex<T, L, R, P>
where
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy + Default,
{
    /// Creates a new `Sharex` lock protecting the given `value`.
    pub fn new(value: T) -> Self
    {
        Self {
            inner:    UnsafeCell::new(value),
            lock:     L::default(),
            retry:    R::default(),
            poisoned: P::default(),
        }
    }
}

#[cfg(dev)]
impl<T, L, R, P> ForceUnlock for Sharex<T, L, R, P>
where
    L: SharingPolicy + ForceUnlock,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    unsafe fn force_unlock(&self)
    {
        unsafe {
            self.lock.force_unlock();
        }
    }
}

impl<T, L, R, P> Sharex<T, L, R, P>
where
    L: SharingPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    /// Attempts to acquire a shared (read) lock without blocking.
    pub fn try_read(&self) -> SharexTryReadResult<'_, T, L, P>
    {
        match self.lock.try_share(0)
        {
            Ok(LockStatus::Done(meta)) =>
            {
                let guard = ShGuard::new(
                    self.inner.get(),
                    &self.lock,
                    meta,
                    &self.poisoned,
                );
                if P::is_poisoned(&self.poisoned)
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
    pub fn try_write(&self) -> SharexTryWriteResult<'_, T, L, P>
    {
        match match unsafe { self.lock.try_lock(0) }
        {
            Ok(LockStatus::Done(meta)) =>
            {
                let guard = ExGuard::new(
                    self.inner.get(),
                    &self.lock,
                    meta,
                    &self.poisoned,
                );
                if P::is_poisoned(&self.poisoned)
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
        {
            Ok(ok) => Ok(ok),
            Err(e) =>
            {
                self.lock.abort();
                Err(e)
            },
        }
    }

    /// Acquires a shared (read) lock, blocking the current thread until it is
    /// available.
    pub fn read(&self) -> SharexReadResult<'_, T, L, R, P>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;
            match self.lock.try_share(iterations)
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    let guard = ShGuard::new(
                        self.inner.get(),
                        &self.lock,
                        meta,
                        &self.poisoned,
                    );
                    if P::is_poisoned(&self.poisoned)
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
    pub fn write(&self) -> SharexWriteResult<'_, T, L, R, P>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;
            match unsafe { self.lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    let guard = ExGuard::new(
                        self.inner.get(),
                        &self.lock,
                        meta,
                        &self.poisoned,
                    );
                    if P::is_poisoned(&self.poisoned)
                    {
                        self.lock.abort();

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
                        self.lock.abort();

                        return Err(AcquireError::Retry(e));
                    }
                },
                Err(e) =>
                {
                    self.lock.abort();

                    return Err(AcquireError::Lock(e))
                },
            }
        }
    }

    /// Exchanges the protected value with `new_value`, returning the old value.
    pub fn exchange(&self, new_value: T)
    -> SharexExchangeResult<'_, T, L, R, P>
    {
        let guard = self.write()?;
        Ok(guard.exchange(new_value))
    }

    /// Non‑blocking version of [`exchange`](Self::exchange).
    pub fn try_exchange(
        &self,
        new_value: T,
    ) -> SharexTryExchangeResult<'_, T, L, P>
    {
        Ok(self.try_write()?.exchange(new_value))
    }

    /// Returns `true` if the lock is poisoned.
    pub fn is_poisoned(&self) -> bool
    {
        P::is_poisoned(&self.poisoned)
    }

    /// Clears the poisoned state of the lock.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the protected data has been manually
    /// repaired or validated before calling this method.
    pub unsafe fn clear_poison(&self)
    {
        unsafe {
            P::clear_poison(&self.poisoned);
        }
    }
}

impl<T, L, R, P> Sharex<T, L, R, P>
where
    T: Default,
    L: SharingPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    /// Takes the value out of the lock, leaving a `Default::default()` value in
    /// its place.
    pub fn take(&self) -> SharexExchangeResult<'_, T, L, R, P>
    {
        Ok(self.write()?.take())
    }

    /// Non‑blocking version of a `take` operation.
    pub fn try_take(&self) -> SharexTryExchangeResult<'_, T, L, P>
    {
        Ok(self.try_write()?.take())
    }
}

impl<'a, T, L, R, P>
    crate::api::Mutex<
        'a,
        T,
        ExGuard<'a, T, L, P>,
        TryLockError<ExGuard<'a, T, L, P>, L::Error>,
        AcquireError<ExGuard<'a, T, L, P>, L::Error, <R as RetryPolicy>::Error>,
    > for Sharex<T, L, R, P>
where
    T: core::fmt::Debug,
    L: SharingPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    fn lock(&'a self) -> SharexWriteResult<'a, T, L, R, P>
    {
        self.write()
    }

    fn try_lock(&'a self) -> SharexTryWriteResult<'a, T, L, P>
    {
        self.try_write()
    }
}

impl<'a, T, L, R, P>
    crate::api::Sharex<
        'a,
        T,
        ShGuard<'a, T, L, P>,
        TryLockError<ShGuard<'a, T, L, P>, L::Error>,
        AcquireError<ShGuard<'a, T, L, P>, L::Error, <R as RetryPolicy>::Error>,
    > for Sharex<T, L, R, P>
where
    T: core::fmt::Debug,
    L: SharingPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    fn read(&'a self) -> SharexReadResult<'a, T, L, R, P>
    {
        self.read()
    }

    fn try_read(&'a self) -> SharexTryReadResult<'a, T, L, P>
    {
        self.try_read()
    }
}
