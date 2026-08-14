//! Shared (read‑write) lock primitives and the core `IShare` trait.

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
pub trait IShare: ILock
{
    /// Attempt to acquire the lock for reading (shared access).
    ///
    /// The `current_iteration` parameter indicates how many times the caller
    /// has already attempted acquisition.
    ///
    /// # Returns
    /// - `Ok(LockStatus::Done)`: reader lock acquired
    /// - `Ok(LockStatus::Fail)`: writer holds the lock
    /// - `Err(e)`: unrecoverable error
    fn try_share(&self, current_iteration: usize) -> LockResult<Self::Error>;

    /// Release a previously acquired shared (reader) lock.
    ///
    /// # Safety
    /// Must only be called when the current thread holds a reader lock.
    fn free_share(&self);

    /// Wake all threads waiting for a shared (reader) lock.
    ///
    /// Default implementation is a no‑op.
    fn wake_readers(&self) {}
}
