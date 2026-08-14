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
pub(crate) mod os;

pub use atomic::*;
pub use nested::*;
pub use os::*;

/// Default number of spin iterations before a lock implementation
/// should consider parking the current thread via an OS primitive.
///
/// Lock implementations may use this constant as the threshold for
/// transitioning from spinning to parking.
pub const DEFAULT_EPSILON: usize = 1 << 13;

/// Default lock strategy for current environment,
/// selected by Resync.
#[cfg(feature = "std")]
pub type DefaultLock = Os;

/// Default lock strategy for current environment,
/// selected by Resync.
#[cfg(not(feature = "std"))]
pub type DefaultLock = Atomic;

use crate::LockResult;

/// A trait for lock primitives that can be atomically acquired and released.
///
/// # Required Methods
/// - [`ILock::try_lock`]: attempt to acquire the lock (may park).
/// - [`ILock::free`]: release the lock (idempotent).
/// - [`ILock::fake_lock`]: check lock state without modifying it.
///
/// # Errors
/// [`ILock::try_lock`] returns a [`LockResult`]:
/// - [`LockResult::Done`]  – lock was successfully acquired.
/// - [`LockResult::Fail`]  – lock was already held.
/// - [`LockResult::Abort`] – an unrecoverable error occurred.
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
    /// The `current_iteration` parameter indicates how many times the caller
    /// has already attempted to acquire the lock. Implementations may use
    /// this to decide whether to park the current thread (e.g., via futex)
    /// when the iteration count exceeds a threshold like [`DEFAULT_EPSILON`].
    ///
    /// # Returns
    /// A [`LockResult`] indicating success, failure, or abort.
    fn try_lock(&self, current_iteration: usize) -> LockResult;

    /// Release the lock.
    ///
    /// This method is idempotent. For futex‑based implementations, it may
    /// wake one waiting thread.
    fn free(&self);

    /// Check the lock state without modifying it.
    ///
    /// This method must never park the current thread.
    ///
    /// # Returns
    /// A [`LockResult`] indicating whether the lock appears free.
    fn fake_lock(&self) -> LockResult;

    /// Wake all threads waiting on this lock.
    ///
    /// The default implementation is a no‑op. Futex‑based implementations
    /// should override this to broadcast a wake to all waiters. This is
    /// used by primitives like [`Gate`](crate::Gate) that need to release
    /// multiple waiters simultaneously.
    fn wake_all(&self) {}
}
