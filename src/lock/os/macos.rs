//! A macOS pthread‑based reader‑writer lock.
//!
//! This implementation uses POSIX `pthread_rwlock_t`, which provides both
//! exclusive (writer) and shared (reader) access.
//!
//! # Performance
//!
//! `pthread_rwlock_t` is optimized for macOS and uses kernel support
//! (via `__psynch_rw_longrdlock` and similar) for efficient parking
//! when contended.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{ILock, IShare, LockResult};

/// macOS pthread‑based reader‑writer lock.
///
/// This type implements both [`ILock`] (writer access) and [`IShare`]
/// (reader access) using POSIX `pthread_rwlock_t`.
#[allow(missing_debug_implementations)]
pub struct Os
{
    rwlock: core::cell::UnsafeCell<libc::pthread_rwlock_t>,
}

impl Os
{
    /// Creates a new unlocked [`Os`] lock.
    pub fn new() -> Self
    {
        let rwlock = core::cell::UnsafeCell::new(unsafe {
            let mut rwlock: libc::pthread_rwlock_t = core::mem::zeroed();
            let result =
                libc::pthread_rwlock_init(&mut rwlock, core::ptr::null());
            debug_assert_eq!(result, 0, "pthread_rwlock_init failed");
            rwlock
        });

        Self { rwlock }
    }
}

impl core::default::Default for Os
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl Drop for Os
{
    fn drop(&mut self)
    {
        unsafe {
            libc::pthread_rwlock_destroy(self.rwlock.get());
        }
    }
}

unsafe impl Send for Os {}
unsafe impl Sync for Os {}

unsafe impl ILock for Os
{
    /// Attempts to acquire an exclusive (writer) lock.
    ///
    /// Uses `pthread_rwlock_trywrlock`, which returns immediately without
    /// blocking if the lock is held.
    fn try_lock(&self, _current_iteration: usize) -> LockResult
    {
        let result =
            unsafe { libc::pthread_rwlock_trywrlock(self.rwlock.get()) };

        if result == 0
        {
            LockResult::Done
        }
        else
        {
            LockResult::Fail
        }
    }

    fn fake_lock(&self) -> LockResult
    {
        // pthread_rwlock doesn't provide a non-modifying check
        LockResult::Done
    }

    /// Releases an exclusive (writer) lock.
    fn free(&self)
    {
        unsafe {
            libc::pthread_rwlock_unlock(self.rwlock.get());
        }
    }

    fn wake_all(&self)
    {
        // pthread_rwlock handles waking automatically
    }
}

impl IShare for Os
{
    /// Attempts to acquire a shared (reader) lock.
    ///
    /// Uses `pthread_rwlock_tryrdlock`, which returns immediately without
    /// blocking if a writer holds the lock.
    fn try_share(&self, _current_iteration: usize) -> LockResult
    {
        let result =
            unsafe { libc::pthread_rwlock_tryrdlock(self.rwlock.get()) };

        if result == 0
        {
            LockResult::Done
        }
        else
        {
            LockResult::Fail
        }
    }

    /// Releases a shared (reader) lock.
    fn free_share(&self)
    {
        unsafe {
            libc::pthread_rwlock_unlock(self.rwlock.get());
        }
    }

    fn wake_readers(&self)
    {
        // pthread_rwlock handles waking automatically
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn os_writer_acquires()
    {
        let lock = Os::new();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        assert_eq!(lock.try_lock(0), LockResult::Fail);
        lock.free();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        lock.free();
    }

    #[test]
    fn os_multiple_readers()
    {
        let lock = Os::new();
        assert_eq!(lock.try_share(0), LockResult::Done);
        assert_eq!(lock.try_share(0), LockResult::Done);
        assert_eq!(lock.try_share(0), LockResult::Done);
        lock.free_share();
        lock.free_share();
        lock.free_share();
    }

    #[test]
    fn os_writer_blocks_readers()
    {
        let lock = Os::new();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        assert_eq!(lock.try_share(0), LockResult::Fail);
        lock.free();
        assert_eq!(lock.try_share(0), LockResult::Done);
        lock.free_share();
    }

    #[test]
    fn os_readers_block_writer()
    {
        let lock = Os::new();
        assert_eq!(lock.try_share(0), LockResult::Done);
        assert_eq!(lock.try_lock(0), LockResult::Fail);
        lock.free_share();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        lock.free();
    }
}
