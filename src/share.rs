//! Shared (read‑write) lock primitives and the core `IShare` trait.
//!
//! This module defines the [`IShare`] trait, which represents a lock that can
//! be acquired either for shared (read) or exclusive (write) access.
//!
//! # Usage
//! Implementors of [`IShare`] can be used as the locking backend for
//! higher‑level primitives like [`RwLock`](crate::RwLock).
//!
//! # Provided Implementations
//! - [`Atomic`]: a reader‑writer lock based on an atomic counter.

mod atomic;
mod os;

pub use atomic::*;
pub use os::*;

/// Default share strategy for current environment,
/// selected by Resync. Good option if you just
/// writing something platform-aware without
/// deep-minding about synchronization.
#[cfg(feature = "std")]
pub type DefaultShare = Atomic;

/// Default share strategy for current environment,
/// selected by Resync. Good option if you just
/// writing something platform-aware without
/// deep-minding about synchronization.
#[cfg(not(feature = "std"))]
pub type DefaultShare = Atomic;

use crate::LockResult;

/// A trait for shared‑exclusive lock primitives.
///
/// This trait is analogous to [`ILock`](crate::ILock), but provides separate
/// methods for read (shared) and write (exclusive) acquisition.
///
/// # Required Methods
/// - [`IShare::try_read`] – attempt to acquire the lock for reading.
/// - [`IShare::try_write`] – attempt to acquire the lock for writing.
/// - [`IShare::free_read`] – release a read lock.
/// - [`IShare::free_write`] – release a write lock.
///
/// # Errors
/// Both `try_read` and `try_write` return a [`LockResult`]:
/// - [`LockResult::Done`]  – the lock was successfully acquired.
/// - [`LockResult::Fail`]  – the lock is currently held in a conflicting mode.
/// - [`LockResult::Abort`] – an unrecoverable error occurred (e.g.,
///   system‑level failure). This implementation never returns `Abort`.
///
/// # Panics
/// Implementations **must not** panic under normal conditions.
pub trait IShare
where Self: core::default::Default
{
    /// Attempt to acquire the lock for reading (shared access).
    ///
    /// This operation must be performed atomically.
    ///
    /// # Returns
    /// A [`LockResult`] indicating success, failure, or abort.
    fn try_read(&self) -> LockResult;

    /// Attempt to acquire the lock for writing (exclusive access).
    ///
    /// This operation must be performed atomically.
    ///
    /// # Returns
    /// A [`LockResult`] indicating success, failure, or abort.
    fn try_write(&self) -> LockResult;

    /// Release a previously acquired read lock.
    ///
    /// # Safety
    /// This method must only be called when the current thread holds a read
    /// lock on this instance. Calling it without holding a read lock may lead
    /// to undefined behaviour.
    fn free_read(&self);

    /// Release a previously acquired write lock.
    ///
    /// # Safety
    /// This method must only be called when the current thread holds the
    /// write lock on this instance. Calling it without holding the write lock
    /// may lead to undefined behaviour.
    fn free_write(&self);
}
