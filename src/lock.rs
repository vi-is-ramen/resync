//! Lock primitives and the core lock trait.
//!
//! This module defines the [`ILock`] trait and provides several concrete
//! implementations:
//! - [`Atomic`]: a lock based on a single atomic boolean.
//! - [`Nested`]: a composite lock that acquires two inner locks in order.
//!
//! # Usage
//! Implementors of [`ILock`] can be used as the locking backend for
//! higher‑level primitives like [`Mutex`](crate::Mutex).

mod atomic;
mod nested;

pub use atomic::*;
pub use nested::*;

// TODO: Os lock variant

use crate::LockResult;

/// A trait for lock primitives that can be atomically acquired and released.
///
/// # Required Methods
/// - [`ILock::try_lock`]: attempt to acquire the lock.
/// - [`ILock::free`]: release the lock (idempotent).
///
/// # Errors
/// [`ILock::try_lock`] returns a [`LockResult`]:
/// - [`LockResult::Done`]  – lock was successfully acquired.
/// - [`LockResult::Fail`]  – lock was already held.
/// - [`LockResult::Abort`] – an unrecoverable error occurred (e.g.,
///   system‑level failure, poisonous lock et al).
///
/// # Panics
/// Implementations **must not** panic under normal conditions.
pub trait ILock
where Self: core::default::Default
{
    /// Attempt to acquire the lock.
    ///
    /// This operation must be performed atomically.
    ///
    /// # Returns
    /// A [`LockResult`] indicating success, failure, or abort.
    fn try_lock(&self) -> LockResult;

    /// Release the lock.
    ///
    /// This method is idempotent – calling it on an already‑free lock does
    /// nothing. It must be safe to call concurrently (though races are
    /// benign because the lock state is set to unlocked).
    fn free(&self);
}
