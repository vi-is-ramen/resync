//! A generic trait for shared lock primitives.
//!
//! This module defines the [`Sharex`] trait, which abstracts over any
//! synchronization primitive that provides shared (reader) access to a
//! protected value of type `T`.
//!
//! # Purpose
//!
//! In `resync`, the concrete [`Sharex`](crate::Sharex) struct is heavily
//! parameterized by its underlying policies. While this provides immense
//! flexibility, it makes writing generic code that accepts "any read-write
//! lock" difficult, as the type signatures become extremely complex.
//!
//! The [`Sharex`] trait solves this by defining a common behavioral contract.
//! It allows library authors and users to write generic algorithms or data
//! structures that require shared access to data, without caring whether the
//! underlying implementation is an atomic spinlock, an OS futex, or a
//! read-write lock.
//!
//! # Dyn-Compatibility
//!
//! This trait is designed to be **dyn-compatible** (object-safe). The error
//! types are separated from the guard type, allowing trait objects like
//! `&dyn Sharex<'_, T, G, TryE, E>` to be used in generic contexts.

use core::fmt::Display;

use crate::api::Guard;

/// A generic trait for shared lock primitives.
///
/// This trait abstracts the core locking behavior of a shared (read-write)
/// lock, decoupling the *action* of locking from the specific *implementation*
/// (e.g., atomic, futex, OS-level). Unlike the previous design that accepted
/// arbitrary result types, this version returns a standard [`Result`] wrapping
/// the guard type `G`, with the error types explicitly parameterized.
///
/// # Type Parameters
///
/// * `'a`: The lifetime of the lock acquisition and the resulting guard.
/// * `T`: The type of the data being protected by the lock.
/// * `G`: The RAII guard type returned on successful acquisition. Must
///   implement [`Guard<T>`], providing shared (read) access to the protected
///   data.
/// * `TryE`: The error type returned by the non-blocking [`try_read`] method.
///   Must implement [`Display`]. Typically represents contention, poisoning, or
///   a fatal lock policy failure.
/// * `E`: The error type returned by the blocking [`read`] method. Must
///   implement [`Display`]. Typically represents poisoning, a fatal lock policy
///   failure, or a retry policy abortion (e.g., timeout).
///
/// # Trait Bounds
///
/// * `Self: Sync` — The lock must be safely shareable across threads.
/// * `G: Guard<T>` — The guard must provide shared access to `T`.
/// * `TryE: Display` — The try-read error.
/// * `E: Display` — The read error.
///
/// # Dyn-Compatibility
///
/// This trait is designed to be dyn-compatible (object-safe), allowing it to
/// be used as `&dyn Sharex<'a, T, G, TryE, E>` in generic code.
///
/// [`try_read`]: Sharex::try_read
/// [`read`]: Sharex::read
// NOTE: This trait **must** be dyn-compatible by design.
pub trait Sharex<'a, T, G, TryE, E>
where
    Self: Sync,
    G: Guard<T>,
    TryE: Display,
    E: Display,
{
    /// Attempts to acquire shared (read) access to the protected data without
    /// blocking.
    ///
    /// # Returns
    ///
    /// * `Ok(G)`: The RAII guard providing shared access to the protected data.
    /// * `Err(TryE)`: An error indicating contention, poisoning, or a fatal
    ///   lock policy failure.
    fn try_read(&'a self) -> Result<G, TryE>;

    /// Acquires shared (read) access to the protected data, blocking the
    /// current thread until it is available.
    ///
    /// # Returns
    ///
    /// * `Ok(G)`: The RAII guard providing shared access to the protected data.
    /// * `Err(E)`: An error indicating poisoning, a fatal lock policy failure,
    ///   or a retry policy abortion (e.g., timeout).
    fn read(&'a self) -> Result<G, E>;
}

#[cfg(any(feature = "std", docsrs))]
impl<'a, T>
    Sharex<
        'a,
        T,
        std::sync::RwLockReadGuard<'a, T>,
        std::sync::TryLockError<std::sync::RwLockReadGuard<'a, T>>,
        std::sync::PoisonError<std::sync::RwLockReadGuard<'a, T>>,
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
