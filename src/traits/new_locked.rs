//! A trait for lock policies that can be initialized in the locked (acquired)
//! state.
//!
//! This module provides the [`NewLocked`] trait, which extends [`LockPolicy`]
//! to support the creation of lock instances that are already acquired upon
//! construction.
//!
//! # Purpose
//!
//! Some synchronization primitives (e.g., gates, barriers, or one-shot latches)
//! must start in a closed or locked state to prevent TOCTOU (Time-of-Check to
//! Time-of-Use) races that would occur if they were created unlocked and then
//! immediately locked.
//!
//! By segregating this capability into a separate trait, `resync` follows the
//! **Interface Segregation Principle**: basic lock policies only need to
//! implement [`LockPolicy`], while policies that support locked initialization
//! additionally implement [`NewLocked`].
//!
//! # Examples
//!
//! ```rust
//! # use resync::traits::{LockPolicy, NewLocked};
//! # use resync::lock::Atomic;
//! # use resync::LockStatus;
//!
//! // Create an Atomic lock that is already acquired.
//! let (meta, lock) = Atomic::new_locked();
//!
//! // Any subsequent attempt to acquire it will fail.
//! assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Fail);
//!
//! // Release it using the returned metadata.
//! unsafe { lock.free(&meta) };
//! ```
use crate::traits::LockPolicy;

/// A lock policy that can be initialized in the **locked** (acquired) state.
///
/// This trait extends [`LockPolicy`] to provide a constructor that returns a
/// lock instance that is already acquired. The caller receives both the lock
/// itself and the [`Meta`](LockPolicy::Meta) data required to eventually
/// release it via [`LockPolicy::free`].
///
/// # Correctness
///
/// Implementations must guarantee that the returned lock is fully acquired such
/// that any subsequent call to [`LockPolicy::try_lock`] by another thread will
/// correctly observe contention (returning [`crate::LockStatus::Fail`] or
/// blocking/parking, depending on the adaptive strategy).
// NOTE: This trait **must not** be dyn-compatible by design.
pub trait NewLocked: LockPolicy
{
    /// Creates a new instance of the lock policy in the **locked** (acquired)
    /// state.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// 1. The [`Meta`](LockPolicy::Meta) data required to eventually release
    ///    the lock via [`LockPolicy::free`].
    /// 2. The initialized lock policy instance itself.
    fn new_locked() -> (Self::Meta, Self);
}
