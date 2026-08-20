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
//!
//! # Dyn-Compatibility
//!
//! This trait is designed to be **dyn-compatible** (object-safe). The error
//! types are separated from the guard type, allowing trait objects like
//! `&dyn Mutex<'_, T, G, TryE, E>` to be used in generic contexts.

use core::fmt::Display;

use crate::api::GuardMut;

/// A generic trait for mutual exclusion primitives.
///
/// This trait abstracts the core locking behavior of a mutex, decoupling the
/// *action* of locking from the specific *implementation* (e.g., atomic,
/// futex, OS-level). This trait returns a standard [`Result`] wrapping the
/// guard type `G`, with the error types explicitly parameterized.
///
/// # Type Parameters
///
/// * `'a`: The lifetime of the lock acquisition and the resulting guard.
/// * `T`: The type of the data being protected by the mutex.
/// * `G`: The RAII guard type returned on successful acquisition. Must
///   implement [`GuardMut<T>`], providing mutable access to the protected data
///   via [`DerefMut`](core::ops::DerefMut).
/// * `TryE`: The error type returned by the non-blocking [`try_lock`] method.
///   Must implement [`Display`]. Typically represents contention, poisoning, or
///   a fatal lock policy failure.
/// * `E`: The error type returned by the blocking [`lock`] method. Must
///   implement [`Display`]. Typically represents poisoning, a fatal lock policy
///   failure, or a retry policy abortion (e.g., timeout).
///
/// # Trait Bounds
///
/// * `Self: Sync` — The mutex must be safely shareable across threads.
/// * `G: GuardMut<T>` — The guard must provide mutable access to `T`.
/// * `TryE: Display` — The try-lock error must be a standard error type.
/// * `E: Display` — The lock error must be a standard error type.
///
/// # Dyn-Compatibility
///
/// This trait is designed to be dyn-compatible (object-safe), allowing it to
/// be used as `&dyn Mutex<'a, T, G, TryE, E>` in generic code.
///
/// [`try_lock`]: Mutex::try_lock
/// [`lock`]: Mutex::lock
// NOTE: This trait **must** be dyn-compatible by design.
pub trait Mutex<'a, T, G, TryE, E>
where
    Self: Sync,
    G: GuardMut<T>,
    TryE: Display,
    E: Display,
{
    /// Attempts to acquire exclusive access to the protected data without
    /// blocking.
    ///
    /// # Returns
    ///
    /// * `Ok(G)`: The RAII guard providing mutable access to the protected
    ///   data.
    /// * `Err(TryE)`: An error indicating contention, poisoning, or a fatal
    ///   lock policy failure.
    fn try_lock(&'a self) -> Result<G, TryE>;

    /// Acquires exclusive access to the protected data, blocking the current
    /// thread until it is available.
    ///
    /// # Returns
    ///
    /// * `Ok(G)`: The RAII guard providing mutable access to the protected
    ///   data.
    /// * `Err(E)`: An error indicating poisoning, a fatal lock policy failure,
    ///   or a retry policy abortion (e.g., timeout).
    fn lock(&'a self) -> Result<G, E>;
}

#[cfg(any(std, docsrs))]
impl<'a, T>
    Mutex<
        'a,
        T,
        std::sync::MutexGuard<'a, T>,
        std::sync::TryLockError<std::sync::MutexGuard<'a, T>>,
        std::sync::PoisonError<std::sync::MutexGuard<'a, T>>,
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
