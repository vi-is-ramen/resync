//! Lock primitives and the core lock trait.
//!
//! This module defines the [`ILock`] trait and provides several concrete
//! implementations.
//!
//! # Usage
//! Implementors of [`ILock`] can be used as the locking backend for
//! higher‑level primitives like [`Mutex`](crate::Mutex).

mod atomic;
mod nested;

pub use crate::os::lock::*;
pub use atomic::*;
pub use nested::*;

/// Default lock strategy for current environment,
/// selected by Resync. Good option if you just
/// writing something platform-aware without
/// deep-minding about synchronization.
#[cfg(feature = "std")]
pub type DefaultLock = Os;

/// Default lock strategy for current environment,
/// selected by Resync. Good option if you just
/// writing something platform-aware without
/// deep-minding about synchronization.
#[cfg(not(feature = "std"))]
pub type DefaultLock = Atomic;

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
///
/// # Safety
///
/// This trait marked `unsafe` as Rust can't guarantee some invariants
/// but developers of implementors must do it.
pub unsafe trait ILock
where Self: core::default::Default
{
    /// Attempt to acquire the lock.
    ///
    /// This operation must be performed atomically for the resource.
    ///
    /// > **[!NOTE]**
    /// > this method is ALLOWED to yield (e. g. wait on futex).
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

    /// Attempt to acquire the lock but without change of its state.
    ///
    /// This operation must be performed atomically.
    ///
    /// > **[!NOTE]**
    /// > this method is **NOT ALLOWED** to yield (e. g. wait on futex).
    ///
    /// # Returns
    /// A [`LockResult`] indicating success, failure or abort.
    fn fake_lock(&self) -> LockResult;
}
