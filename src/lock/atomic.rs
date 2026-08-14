//! An atomic counter‑based lock that implements both [`ILock`] (exclusive
//! writer access) and [`IShare`] (shared reader access).
//!
//! # State Encoding
//! - `0`: the lock is free.
//! - `1..=usize::MAX-1`: the lock is held by that many readers.
//! - `usize::MAX`: the lock is held by a writer.
//!
//! This single type can be used as the backend for both [`Mutex`] and
//! [`RwLock`].

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{ILock, IShare, LockResult};

/// The sentinel value indicating that a writer holds the lock.
const WRITER: usize = usize::MAX;

/// A lock that uses a single [`AtomicUsize`] as its underlying state,
/// supporting both exclusive (writer) and shared (reader) access.
///
/// # Examples
/// ```rust
/// # use resync::{ILock, IShare, LockResult};
/// use resync::lock::Atomic;
///
/// let lock = Atomic::new();
///
/// // Writer access
/// assert_eq!(lock.try_lock(0), LockResult::Done);
/// assert_eq!(lock.try_lock(0), LockResult::Fail); // already held
/// lock.free();
///
/// // Reader access
/// assert_eq!(lock.try_share(0), LockResult::Done);
/// assert_eq!(lock.try_share(0), LockResult::Done); // multiple readers OK
/// assert_eq!(lock.try_lock(0), LockResult::Fail); // writer blocked
/// lock.free_share();
/// lock.free_share();
/// ```
#[allow(missing_debug_implementations)]
pub struct Atomic(AtomicUsize);

impl Atomic
{
    /// Creates a new unlocked [`Atomic`] lock.
    pub const fn new() -> Self
    {
        Self(AtomicUsize::new(0))
    }
}

impl core::default::Default for Atomic
{
    fn default() -> Self
    {
        Self::new()
    }
}

unsafe impl ILock for Atomic
{
    /// Attempts to acquire an exclusive (writer) lock.
    ///
    /// Succeeds only if the lock is currently free (`state == 0`). The
    /// `current_iteration` parameter is ignored; this lock never parks.
    fn try_lock(&self, _current_iteration: usize) -> LockResult
    {
        match self.0.compare_exchange(
            0,
            WRITER,
            Ordering::Acquire,
            Ordering::Relaxed,
        )
        {
            Ok(_) => LockResult::Done,
            Err(_) => LockResult::Fail,
        }
    }

    fn fake_lock(&self) -> LockResult
    {
        if self.0.load(Ordering::Relaxed) == 0
        {
            LockResult::Done
        }
        else
        {
            LockResult::Fail
        }
    }

    /// Releases an exclusive (writer) lock by resetting the state to `0`.
    ///
    /// This method is idempotent.
    fn free(&self)
    {
        self.0.store(0, Ordering::Release);
    }
}

impl IShare for Atomic
{
    /// Attempts to acquire a shared (reader) lock.
    ///
    /// Succeeds if no writer holds the lock. Increments the reader count.
    /// The `current_iteration` parameter is ignored; this lock never parks.
    fn try_share(&self, _current_iteration: usize) -> LockResult
    {
        loop
        {
            let state = self.0.load(Ordering::Relaxed);
            if state == WRITER
            {
                return LockResult::Fail;
            }

            match self.0.compare_exchange_weak(
                state,
                state + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            {
                Ok(_) => return LockResult::Done,
                Err(_) => continue, // CAS failed, retry
            }
        }
    }

    /// Releases a shared (reader) lock by decrementing the reader count.
    fn free_share(&self)
    {
        self.0.fetch_sub(1, Ordering::Release);
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
        assert_eq!(lock.try_lock(0), LockResult::Done);
        lock.free();
    }

    #[test]
    fn atomic_default_is_unlocked()
    {
        let lock = Atomic::default();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        lock.free();
    }

    #[test]
    fn atomic_writer_blocks_writer()
    {
        let lock = Atomic::new();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        assert_eq!(lock.try_lock(0), LockResult::Fail);
        lock.free();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        lock.free();
    }

    #[test]
    fn atomic_multiple_readers_ok()
    {
        let lock = Atomic::new();
        assert_eq!(lock.try_share(0), LockResult::Done);
        assert_eq!(lock.try_share(0), LockResult::Done);
        assert_eq!(lock.try_share(0), LockResult::Done);
        lock.free_share();
        lock.free_share();
        lock.free_share();
    }

    #[test]
    fn atomic_writer_blocks_readers()
    {
        let lock = Atomic::new();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        assert_eq!(lock.try_share(0), LockResult::Fail);
        lock.free();
        assert_eq!(lock.try_share(0), LockResult::Done);
        lock.free_share();
    }

    #[test]
    fn atomic_readers_block_writer()
    {
        let lock = Atomic::new();
        assert_eq!(lock.try_share(0), LockResult::Done);
        assert_eq!(lock.try_lock(0), LockResult::Fail);
        lock.free_share();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        lock.free();
    }

    #[test]
    fn atomic_free_is_idempotent()
    {
        let lock = Atomic::new();
        lock.free();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        lock.free();
        lock.free();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        lock.free();
    }
}
