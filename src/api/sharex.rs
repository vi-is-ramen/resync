//! A generic trait for shared lock primitives.
//!
//! This module defines the [`Sharex`] trait, which abstracts over any
//! synchronization primitive that provides shared access to a protected
//! value of type `T`.
//!
//! # Purpose
//!
//! While the concrete [`Sharex`](crate::Sharex) struct is heavily parameterized
//! by its underlying [`SharingPolicy`](crate::traits::SharingPolicy) and
//! [`RetryPolicy`](crate::traits::RetryPolicy), this trait allows library
//! authors to write generic algorithms or data structures that require
//! read-write access to data, without caring about the specific backend or
//! error handling strategy.

/// A generic trait for shared lock primitives.
///
/// This trait abstracts the core locking behavior of a shared lock,
/// decoupling the *action* of locking from the specific *implementation* and
/// the specific *error types* returned by the lock and retry policies.
///
/// # Type Parameters
///
/// * `'a`: The lifetime of the lock acquisition and the resulting guard.
/// * `T`: The type of the data being protected by the lock.
/// * `TryR`: The result type returned by the non-blocking [`try_read`] method.
/// * `R`: The result type returned by the blocking [`read`] method.
///
/// [`try_read`]: Sharex::try_read
/// [`read`]: Sharex::read
pub trait Sharex<'a, T, TryR, R>: Sync
{
    /// Attempts to acquire shared (read) access to the protected data without
    /// blocking.
    ///
    /// # Returns
    ///
    /// The `TryReadR` type, which typically resolves to a shared RAII guard on
    /// success, or an error indicating contention, poisoning, or a fatal lock
    /// policy failure.
    fn try_read(&'a self) -> TryR;

    /// Acquires shared (read) access to the protected data, blocking the
    /// current thread until it is available.
    ///
    /// # Returns
    ///
    /// The `ReadR` type, which typically resolves to a shared RAII guard on
    /// success, or an error indicating poisoning, a fatal lock policy failure,
    /// or a retry policy abortion (e.g., timeout).
    fn read(&'a self) -> R;
}

#[cfg(any(std, docsrs))]
impl<'a, T>
    crate::api::Sharex<
        'a,
        T,
        Result<
            std::sync::RwLockReadGuard<'a, T>,
            std::sync::TryLockError<std::sync::RwLockReadGuard<'a, T>>,
        >,
        Result<
            std::sync::RwLockReadGuard<'a, T>,
            std::sync::PoisonError<std::sync::RwLockReadGuard<'a, T>>,
        >,
    > for std::sync::RwLock<T>
where T: Send + Sync
{
    fn try_read(
        &'a self,
    ) -> Result<
        std::sync::RwLockReadGuard<'a, T>,
        std::sync::TryLockError<std::sync::RwLockReadGuard<'a, T>>,
    >
    {
        self.try_read()
    }

    fn read(
        &'a self,
    ) -> Result<
        std::sync::RwLockReadGuard<'a, T>,
        std::sync::PoisonError<std::sync::RwLockReadGuard<'a, T>>,
    >
    {
        self.read()
    }
}
