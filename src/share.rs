//! Shared (read‑write) lock primitives and the core `IShare` trait.
//!
//! `IShare` extends [`ILock`](crate::ILock) by adding reader (shared) access.
//! The inherited `try_lock`/`free` methods handle exclusive (writer) access,
//! while the new `try_share`/`free_share` methods handle shared (reader)
//! access.

// Re-export the unified Atomic type from the lock module.
pub use crate::lock::Atomic;

/// Default share strategy for current environment.
#[cfg(feature = "std")]
pub type DefaultShare = crate::lock::DefaultLock;

/// Default share strategy for current environment.
#[cfg(not(feature = "std"))]
pub type DefaultShare = Atomic;

use crate::{ILock, LockResult};

/// A trait for shared‑exclusive (reader‑writer) lock primitives.
///
/// `IShare` extends [`ILock`]: the inherited methods `try_lock` and `free`
/// provide exclusive (writer) access, while `try_share` and `free_share`
/// provide shared (reader) access.
///
/// # Required Methods
/// - [`ILock::try_lock`] – attempt to acquire an exclusive (writer) lock.
/// - [`ILock::free`] – release the exclusive lock.
/// - [`IShare::try_share`] – attempt to acquire a shared (reader) lock.
/// - [`IShare::free_share`] – release the shared lock.
///
/// # Errors
/// Both `try_lock` and `try_share` return a [`LockResult`]:
/// - [`LockResult::Done`] – lock acquired successfully.
/// - [`LockResult::Fail`] – lock held in a conflicting mode.
/// - [`LockResult::Abort`] – unrecoverable error.
pub trait IShare: ILock
{
    /// Attempt to acquire the lock for reading (shared access).
    ///
    /// The `current_iteration` parameter indicates how many times the caller
    /// has already attempted acquisition. Implementations may use this to
    /// decide whether to park the current thread.
    ///
    /// Succeeds if no writer currently holds the lock. Multiple readers may
    /// hold the lock concurrently.
    fn try_share(&self, current_iteration: usize) -> LockResult;

    /// Release a previously acquired shared (reader) lock.
    ///
    /// # Safety
    /// Must only be called when the current thread holds a reader lock.
    /// Calling it without holding a reader lock may corrupt the lock state.
    fn free_share(&self);

    /// Wake all threads waiting for a shared (reader) lock.
    ///
    /// Default implementation is a no‑op. Futex‑based implementations should
    /// override this.
    fn wake_readers(&self) {}
}
