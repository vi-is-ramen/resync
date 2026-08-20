//! A mutual exclusion primitive that composes a lock policy and a retry policy.
use super::ExGuard;
use crate::traits::{LockPolicy, PoisonPolicy, RetryPolicy};
use crate::{AcquireError, LockStatus, PoisonError, TryLockError};
use core::cell::UnsafeCell;

/// A mutual exclusion (mutex) primitive that protects a value of type `T`.
#[allow(missing_debug_implementations)]
pub struct Mutex<
    T,
    L = crate::lock::DefaultLock,
    R = crate::retry::DefaultRetry,
    P = crate::poison::DefaultPoison,
> where
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    inner:    UnsafeCell<T>,
    lock:     L,
    retry:    R,
    poisoned: P,
}

impl<T, L, R, P> core::fmt::Debug for Mutex<T, L, R, P>
where
    T: core::fmt::Debug,
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
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

unsafe impl<T, L, R, P> core::marker::Sync for Mutex<T, L, R, P>
where
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
}

unsafe impl<T, L, R, P> core::marker::Send for Mutex<T, L, R, P>
where
    T: core::marker::Send,
    L: LockPolicy + core::marker::Send,
    R: RetryPolicy + core::marker::Send,
    P: PoisonPolicy,
{
}

impl<T, L, R, P> core::default::Default for Mutex<T, L, R, P>
where
    T: Default,
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy + Default,
{
    fn default() -> Self
    {
        Self {
            inner:    UnsafeCell::new(T::default()),
            lock:     L::default(),
            retry:    R::default(),
            poisoned: P::default(),
        }
    }
}

/// Result type for non-blocking [`Mutex::try_lock`] operations.
pub type MutexTryLockResult<'a, T, L, P>
where
    L: LockPolicy,
    P: PoisonPolicy,
= Result<
    ExGuard<'a, T, L, P, L::Meta>,
    TryLockError<ExGuard<'a, T, L, P, L::Meta>, L::Error>,
>;

/// Result type for blocking [`Mutex::lock`] operations.
pub type MutexLockResult<'a, T, L, R, P>
where
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
= Result<
    ExGuard<'a, T, L, P>,
    AcquireError<ExGuard<'a, T, L, P>, L::Error, <R as RetryPolicy>::Error>,
>;

/// Result type for blocking [`Mutex::exchange`] and [`Mutex::take`] operations.
pub type MutexExchangeResult<'a, T, L, R, P>
where
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
= Result<
    T,
    AcquireError<ExGuard<'a, T, L, P>, L::Error, <R as RetryPolicy>::Error>,
>;

/// Result type for non-blocking [`Mutex::try_exchange`] and [`Mutex::try_take`]
/// operations.
pub type MutexTryExchangeResult<'a, T, L, P>
where
    L: LockPolicy,
    P: PoisonPolicy,
= Result<T, TryLockError<ExGuard<'a, T, L, P, L::Meta>, L::Error>>;

impl<T, L, R, P> Mutex<T, L, R, P>
where
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy + Default,
{
    /// Creates a new mutex protecting the given `value`.
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

impl<T, L, R, P> Mutex<T, L, R, P>
where
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    /// Attempts to acquire the mutex without blocking.
    pub fn try_lock(&self) -> MutexTryLockResult<'_, T, L, P>
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
                // lock must know: we will not continue attempts.
                self.lock.abort();
                Err(e)
            },
        }
    }

    /// Acquires the mutex, blocking the current thread until it is available.
    pub fn lock(&self) -> MutexLockResult<'_, T, L, R, P>
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
    -> MutexExchangeResult<'_, T, L, R, P>
    {
        let guard = self.lock()?;
        Ok(guard.exchange(new_value))
    }

    /// Non‑blocking version of [`exchange`](Self::exchange).
    pub fn try_exchange(
        &self,
        new_value: T,
    ) -> MutexTryExchangeResult<'_, T, L, P>
    {
        Ok(self.try_lock()?.exchange(new_value))
    }

    /// Returns `true` if the mutex is poisoned.
    pub fn is_poisoned(&self) -> bool
    {
        P::is_poisoned(&self.poisoned)
    }

    /// Clears the poisoned state of the mutex.
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

impl<T, L, R, P> Mutex<T, L, R, P>
where
    T: Default,
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    /// Takes the value out of the mutex, leaving a `Default::default()` value
    /// in its place.
    pub fn take(&self) -> MutexExchangeResult<'_, T, L, R, P>
    {
        Ok(self.lock()?.take())
    }

    /// Non‑blocking version of [`take`](Self::take).
    pub fn try_take(&self) -> MutexTryExchangeResult<'_, T, L, P>
    {
        Ok(self.try_lock()?.take())
    }
}

impl<'a, T, L, R, P>
    crate::api::Mutex<
        'a,
        T,
        ExGuard<'a, T, L, P>,
        TryLockError<ExGuard<'a, T, L, P>, L::Error>,
        AcquireError<ExGuard<'a, T, L, P>, L::Error, <R as RetryPolicy>::Error>,
    > for Mutex<T, L, R, P>
where
    T: core::fmt::Debug,
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    fn lock(&'a self) -> MutexLockResult<'a, T, L, R, P>
    {
        self.lock()
    }

    fn try_lock(&'a self) -> MutexTryLockResult<'a, T, L, P>
    {
        self.try_lock()
    }
}
