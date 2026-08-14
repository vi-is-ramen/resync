//! Lock primitives and the core lock trait.
//!
//! This module defines the [`ILock`] trait and provides several concrete
//! implementations.

mod atomic;
mod nested;
pub(crate) mod os;

pub use atomic::*;
pub use nested::*;
pub use os::*;

/// Default number of spin iterations before a lock implementation
/// should consider parking the current thread via an OS primitive.
pub const DEFAULT_EPSILON: usize = 1 << 13;

/// Default lock strategy for current environment.
#[cfg(feature = "std")]
pub type DefaultLock = Os;

/// Default lock strategy for current environment.
#[cfg(not(feature = "std"))]
pub type DefaultLock = Atomic;

use crate::LockResult;

/// A trait for lock primitives that can be atomically acquired and released.
///
/// # Associated Types
/// - `Error`: the error type for unrecoverable failures
///
/// # Required Methods
/// - [`ILock::try_lock`]: attempt to acquire the lock (may park).
/// - [`ILock::free`]: release the lock (idempotent).
/// - [`ILock::fake_lock`]: check lock state without modifying it.
///
/// # Returns
/// Lock operations return [`LockResult<Self::Error>`]:
/// - `Ok(LockStatus::Done)`: lock acquired successfully
/// - `Ok(LockStatus::Fail)`: lock is held
/// - `Err(e)`: unrecoverable error
///
/// # Panics
/// Implementations **must not** panic under normal conditions.
///
/// # Safety
///
/// This trait is marked `unsafe` because implementations must uphold
/// synchronization invariants that Rust cannot verify.
pub unsafe trait ILock
where Self: core::default::Default
{
    /// The error type for unrecoverable failures.
    ///
    /// Use `core::convert::Infallible` for locks that never fail.
    type Error;

    /// Attempt to acquire the lock.
    ///
    /// The `current_iteration` parameter indicates how many times the caller
    /// has already attempted to acquire the lock. Implementations may use
    /// this to decide whether to park the current thread when the iteration
    /// count exceeds [`DEFAULT_EPSILON`].
    ///
    /// # Returns
    /// - `Ok(LockStatus::Done)`: lock acquired
    /// - `Ok(LockStatus::Fail)`: lock is held
    /// - `Err(e)`: unrecoverable error
    fn try_lock(&self, current_iteration: usize) -> LockResult<Self::Error>;

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
    /// - `Ok(LockStatus::Done)`: lock appears free
    /// - `Ok(LockStatus::Fail)`: lock appears held
    /// - `Err(e)`: unrecoverable error
    fn fake_lock(&self) -> LockResult<Self::Error>;

    /// Wake all threads waiting on this lock.
    ///
    /// The default implementation is a no‑op. Futex‑based implementations
    /// should override this to broadcast a wake to all waiters.
    fn wake_all(&self) {}
}
