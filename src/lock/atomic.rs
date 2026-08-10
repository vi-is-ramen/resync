//! An atomic boolean‑based lock.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::{ILock, LockResult};

/// A lock that uses a single [`AtomicBool`] as its underlying state.
///
/// # Examples
/// ```rust
/// # use resync::ILock;
/// use resync::LockResult;
/// use resync::lock::Atomic;
///
/// let lock = Atomic::new();
/// assert_eq!(lock.try_lock(), LockResult::Done);
/// assert_eq!(lock.try_lock(), LockResult::Fail);
/// lock.free();
/// assert_eq!(lock.try_lock(), LockResult::Done);
/// ```
#[allow(missing_debug_implementations)]
pub struct Atomic
{
    flag: AtomicBool,
}

impl Atomic
{
    /// Creates a new unlocked [`Atomic`] lock.
    pub const fn new() -> Self
    {
        Self {
            flag: AtomicBool::new(false),
        }
    }
}

#[cfg(nightly)]
const impl core::default::Default for Atomic
{
    fn default() -> Self
    {
        Self {
            flag: AtomicBool::new(false),
        }
    }
}

#[cfg(not(nightly))]
impl core::default::Default for Atomic
{
    fn default() -> Self
    {
        Self {
            flag: AtomicBool::new(false),
        }
    }
}

impl ILock for Atomic
{
    /// Attempts to acquire the lock using a compare‑and‑swap operation.
    ///
    /// # Memory Ordering
    /// - On success: [`Ordering::Acquire`] ordering (ensures subsequent
    ///   operations happen after the lock is acquired).
    /// - On failure: [`Ordering::Relaxed`] ordering (no synchronisation
    ///   needed).
    ///
    /// # Returns
    /// - [`LockResult::Done`]  – lock was successfully acquired.
    /// - [`LockResult::Fail`]  – lock was already held.
    /// - [`LockResult::Abort`] – never returned by this implementation.
    fn try_lock(&self) -> LockResult
    {
        match self.flag.compare_exchange(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed,
        )
        {
            Ok(_) => LockResult::Done,
            Err(_) => LockResult::Fail,
        }
    }

    /// Releases the lock by storing `false` with [`Ordering::Release`]
    /// ordering.
    ///
    /// This method is idempotent.
    fn free(&self)
    {
        self.flag.store(false, Ordering::Release)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn atomic_new_is_unlocked()
    {
        let lock = Atomic::new();
        assert_eq!(lock.try_lock(), LockResult::Done);
    }

    #[test]
    fn atomic_default_is_unlocked()
    {
        let lock = Atomic::default();
        assert_eq!(lock.try_lock(), LockResult::Done);
    }

    #[test]
    fn atomic_try_lock_fails_when_held()
    {
        let lock = Atomic::new();
        assert_eq!(lock.try_lock(), LockResult::Done);
        assert_eq!(lock.try_lock(), LockResult::Fail);
    }

    #[test]
    fn atomic_free_unlocks()
    {
        let lock = Atomic::new();
        assert_eq!(lock.try_lock(), LockResult::Done);
        lock.free();
        assert_eq!(lock.try_lock(), LockResult::Done);
    }

    #[test]
    fn atomic_free_is_idempotent()
    {
        let lock = Atomic::new();
        lock.free(); // already free
        assert_eq!(lock.try_lock(), LockResult::Done);
        lock.free();
        lock.free(); // multiple times
        assert_eq!(lock.try_lock(), LockResult::Done);
    }
}
