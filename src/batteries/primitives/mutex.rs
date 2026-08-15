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

use super::ExGuard;
use crate::traits::{LockPolicy, RetryPolicy};
use crate::{LockError, LockStatus, TryLockError};
use core::cell::UnsafeCell;

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
pub struct Mutex<T, L = crate::lock::Os, R = crate::retry::Yield>
where
    L: LockPolicy,
    R: RetryPolicy,
{
    inner: UnsafeCell<T>,
    lock:  L,
    retry: R,
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

// SAFETY:
// The mutex ensures exclusive access to `T`.
unsafe impl<T, L, R> core::marker::Sync for Mutex<T, L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
}

// SAFETY:
// The mutex can be safely moved between threads as long as `T`, `L` and `R` are
// `Send`.
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
            inner: UnsafeCell::new(T::default()),
            lock:  L::default(),
            retry: R::default(),
        }
    }
}

/// Creates a new `Mutex` from a tuple of the protected value and a custom lock
/// policy.
///
/// The retry policy is initialized using its `Default` implementation.
impl<T, L, R> From<(T, L)> for Mutex<T, L, R>
where
    L: LockPolicy,
    R: RetryPolicy + Default,
{
    fn from(value: (T, L)) -> Self
    {
        Self {
            inner: UnsafeCell::new(value.0),
            lock:  value.1,
            retry: R::default(),
        }
    }
}

/// Creates a new `Mutex` from a tuple of the protected value, a custom lock
/// policy, and a custom retry policy.
impl<T, L, R> From<(T, L, R)> for Mutex<T, L, R>
where
    L: LockPolicy,
    R: RetryPolicy,
{
    fn from(value: (T, L, R)) -> Self
    {
        Self {
            inner: UnsafeCell::new(value.0),
            lock:  value.1,
            retry: value.2,
        }
    }
}

impl<T, L1, R1> Mutex<T, L1, R1>
where
    L1: LockPolicy,
    R1: RetryPolicy,
{
    /// Converts this mutex into a new mutex with different lock and retry
    /// policies.
    ///
    /// This method consumes the current mutex, transfers the protected data
    /// `T`, and returns a new `Mutex` parameterized by the new lock policy
    /// `L2` and retry policy `R2`.
    ///
    /// # Safety
    ///
    /// This operation **does not preserve the lock state**. The new lock and
    /// retry policies are initialized using their `Default` implementations
    /// (i.e., unlocked). Any threads currently waiting on the old lock policy
    /// (`L1`) will **not** be woken up and may deadlock or wait indefinitely,
    /// as the new lock policy (`L2`) has no knowledge of them.
    ///
    /// This method is primarily useful for changing synchronization strategies
    /// at known safe points (e.g., during initialization or when you can
    /// guarantee no other threads are contending for the lock).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use resync::Mutex;
    /// # use resync::lock::Atomic;
    /// # use resync::retry::Busy;
    /// // Start with default Os/Yield policies (requires `std` feature)
    /// let mutex = Mutex::<i32>::new(42);
    ///
    /// // Convert to Atomic/Busy policies
    /// let busy_mutex: Mutex<i32, Atomic, Busy> = unsafe { mutex.to() };
    /// assert_eq!(*busy_mutex.lock().unwrap(), 42);
    /// ```
    pub unsafe fn to<L2, R2>(self) -> Mutex<T, L2, R2>
    where
        L2: LockPolicy + Default,
        R2: RetryPolicy + Default,
    {
        Mutex::<T, L2, R2> {
            inner: self.inner,
            lock:  L2::default(),
            retry: R2::default(),
        }
    }
}

impl<T, L, R> Mutex<T, L, R>
where
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
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
            retry: R::default(),
        }
    }

    /// Attempts to acquire the mutex without blocking.
    ///
    /// This method calls [`LockPolicy::try_lock`] exactly once. If the lock
    /// is currently held by another thread, it returns
    /// `Err(TryLockError::Contention)`. If an unrecoverable lock error occurs,
    /// it returns `Err(TryLockError::Lock(e))`.
    ///
    /// # Returns
    ///
    /// - `Ok(guard)`: The lock was successfully acquired.
    /// - `Err(TryLockError::Contention)`: The lock is currently held by another
    ///   owner.
    /// - `Err(TryLockError::Lock(e))`: An unrecoverable error occurred in the
    ///   lock policy.
    pub fn try_lock(
        &self,
    ) -> Result<ExGuard<'_, T, L, L::Meta>, TryLockError<L::Error>>
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

    /// Acquires the mutex, blocking the current thread until it is available.
    ///
    /// This method repeatedly calls [`LockPolicy::try_lock`]. If the lock is
    /// not immediately available, it calls [`RetryPolicy::retry`] to wait
    /// (e.g., by spinning or yielding) before trying again.
    ///
    /// # Returns
    ///
    /// - `Ok(guard)`: The lock was successfully acquired.
    /// - `Err(LockError::Lock(e))`: An unrecoverable error occurred in the lock
    ///   policy.
    /// - `Err(LockError::Retry(e))`: The retry policy aborted the acquisition
    ///   loop (e.g., due to a timeout).
    pub fn lock(
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
        let guard = self.lock()?;
        Ok(guard.exchange(new_value))
    }

    /// Non‑blocking version of [`exchange`](Self::exchange).
    ///
    /// Attempts to acquire the lock without waiting. If successful, exchanges
    /// the protected value with `new_value`, returns the old value, and
    /// releases the lock.
    ///
    /// # Returns
    ///
    /// - `Ok(old_value)`: The lock was immediately available and the exchange
    ///   succeeded.
    /// - `Err(TryLockError::Contention)`: The lock is currently held by another
    ///   thread.
    /// - `Err(TryLockError::Lock(e))`: An unrecoverable error occurred in the
    ///   lock policy.
    pub fn try_exchange(
        &self,
        new_value: T,
    ) -> Result<T, TryLockError<L::Error>>
    {
        Ok(self.try_lock()?.exchange(new_value))
    }
}

impl<T, L, R> Mutex<T, L, R>
where
    T: Default,
    L: LockPolicy + Default,
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
        Ok(self.lock()?.take())
    }

    /// Non‑blocking version of [`take`](Self::take).
    ///
    /// Attempts to acquire the lock without waiting. If successful, takes the
    /// protected value, replaces it with `Default::default()`, and releases the
    /// lock.
    ///
    /// # Returns
    ///
    /// - `Ok(value)`: The lock was immediately available and the value was
    ///   taken.
    /// - `Err(TryLockError::Contention)`: The lock is currently held by another
    ///   thread.
    /// - `Err(TryLockError::Lock(e))`: An unrecoverable error occurred in the
    ///   lock policy.
    pub fn try_take(&self) -> Result<T, TryLockError<L::Error>>
    {
        Ok(self.try_lock()?.take())
    }
}
