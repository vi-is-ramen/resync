//! A Windows SRWLOCK‑based reader‑writer lock.
//!
//! This implementation uses Windows Slim Reader/Writer Lock (`SRWLOCK`),
//! which provides both exclusive (writer) and shared (reader) access through
//! a single synchronization primitive.
//!
//! # Performance
//!
//! `SRWLOCK` is extremely fast in uncontended scenarios (similar to a
//! `CRITICAL_SECTION` but lighter) and automatically parks waiters via
//! the kernel when contended.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{ILock, IShare, LockResult};

/// Windows SRWLOCK‑based reader‑writer lock.
///
/// This type implements both [`ILock`] (writer access) and [`IShare`]
/// (reader access) using the Windows `SRWLOCK` primitive.
///
/// # Safety
///
/// The inner `SRWLOCK` is initialized to zero (unlocked state) and is
/// safe to use from multiple threads.
#[allow(missing_debug_implementations)]
#[repr(transparent)]
pub struct Os
{
    srwlock: windows_sys::Win32::System::Threading::SRWLOCK,
}

impl Os
{
    /// Creates a new unlocked [`Os`] lock.
    pub fn new() -> Self
    {
        Self {
            srwlock: windows_sys::Win32::System::Threading::SRWLOCK {
                Ptr: core::ptr::null_mut(),
            },
        }
    }
}

impl core::default::Default for Os
{
    fn default() -> Self
    {
        Self::new()
    }
}

unsafe impl Send for Os {}
unsafe impl Sync for Os {}

unsafe impl ILock for Os
{
    /// Attempts to acquire an exclusive (writer) lock.
    ///
    /// Uses `TryAcquireSRWLockExclusive`, which returns immediately without
    /// blocking if the lock is held.
    fn try_lock(&self, _current_iteration: usize) -> LockResult
    {
        let result = unsafe {
            windows_sys::Win32::System::Threading::TryAcquireSRWLockExclusive(
                &self.srwlock as *const _ as *mut _,
            )
        };

        if result != 0
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
        // SRWLOCK doesn't provide a non-modifying check, so we assume it always
        // unlocked. It's much better and doesn't violate invariant "no
        // state change"
        LockResult::Done
    }

    /// Releases an exclusive (writer) lock.
    fn free(&self)
    {
        unsafe {
            windows_sys::Win32::System::Threading::ReleaseSRWLockExclusive(
                &self.srwlock as *const _ as *mut _,
            );
        }
    }

    fn wake_all(&self)
    {
        // SRWLOCK handles waking automatically
    }
}

impl IShare for Os
{
    /// Attempts to acquire a shared (reader) lock.
    ///
    /// Uses `TryAcquireSRWLockShared`, which returns immediately without
    /// blocking if a writer holds the lock.
    fn try_share(&self, _current_iteration: usize) -> LockResult
    {
        let result = unsafe {
            windows_sys::Win32::System::Threading::TryAcquireSRWLockShared(
                &self.srwlock as *const _ as *mut _,
            )
        };

        if result != 0
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
            windows_sys::Win32::System::Threading::ReleaseSRWLockShared(
                &self.srwlock as *const _ as *mut _,
            );
        }
    }

    fn wake_readers(&self)
    {
        // SRWLOCK handles waking automatically
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
