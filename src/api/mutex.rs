//! A generic trait for mutual exclusion primitives.
//!
//! This module defines the [`Mutex`] trait, which abstracts over any
//! synchronization primitive that provides exclusive (writer) access to a
//! protected value of type `T`.
//!
//! # Purpose
//!
//! In `resync`, the concrete [`Mutex`](crate::Mutex) and
//! [`Sharex`](crate::Sharex) structs are heavily parameterized by their
//! underlying policies. While this provides immense flexibility, it makes
//! writing generic code that accepts "any mutex" difficult, as the type
//! signatures become extremely complex.
//!
//! The [`Mutex`] trait solves this by defining a common behavioral contract.
//! It allows library authors and users to write generic algorithms or data
//! structures that require exclusive access to data, without caring whether
//! the underlying implementation is an atomic spinlock, an OS futex, or a
//! read-write lock used in exclusive mode.

/// A generic trait for mutual exclusion primitives.
///
/// This trait abstracts the core locking behavior of a mutex, decoupling the
/// *action* of locking from the specific *implementation* (e.g., atomic,
/// futex, OS-level) and the specific *error types* returned by the lock and
/// retry policies.
///
/// # Type Parameters
///
/// * `'a`: The lifetime of the lock acquisition and the resulting guard.
/// * `T`: The type of the data being protected by the mutex.
/// * `TryR`: The result type returned by the non-blocking [`try_lock`] method.
///   This is typically a [`Result`](core::result::Result) wrapping an RAII
///   guard or a [`TryLockError`](crate::TryLockError).
/// * `R`: The result type returned by the blocking [`lock`] method. This is
///   typically a [`Result`](core::result::Result) wrapping an RAII guard or an
///   [`AcquireError`](crate::AcquireError).
///
/// [`try_lock`]: Mutex::try_lock
/// [`lock`]: Mutex::lock
pub trait Mutex<'a, T, TryR, R>: Sync
{
    /// Attempts to acquire exclusive access to the protected data without
    /// blocking.
    ///
    /// # Returns
    ///
    /// The `TryR` type, which typically resolves to an RAII guard on
    /// success, or an error indicating contention, poisoning, or a fatal
    /// lock policy failure.
    fn try_lock(&'a self) -> TryR;

    /// Acquires exclusive access to the protected data, blocking the current
    /// thread until it is available.
    ///
    /// # Returns
    ///
    /// The `R` type, which typically resolves to an RAII guard on
    /// success, or an error indicating poisoning, a fatal lock policy failure,
    /// or a retry policy abortion (e.g., timeout).
    fn lock(&'a self) -> R;
}

impl<'a, T>
    Mutex<
        'a,
        T,
        Result<
            std::sync::MutexGuard<'a, T>,
            std::sync::TryLockError<std::sync::MutexGuard<'a, T>>,
        >,
        Result<
            std::sync::MutexGuard<'a, T>,
            std::sync::PoisonError<std::sync::MutexGuard<'a, T>>,
        >,
    > for std::sync::Mutex<T>
where T: Send
{
    fn lock(
        &'a self,
    ) -> Result<
        std::sync::MutexGuard<'a, T>,
        std::sync::PoisonError<std::sync::MutexGuard<'a, T>>,
    >
    {
        self.lock()
    }

    fn try_lock(
        &'a self,
    ) -> Result<
        std::sync::MutexGuard<'a, T>,
        std::sync::TryLockError<std::sync::MutexGuard<'a, T>>,
    >
    {
        self.try_lock()
    }
}
