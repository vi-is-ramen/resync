//! A macOS-specific implementation of [`LockPolicy`] and [`SharingPolicy`]
//! using `pthread_rwlock_t`.
//!
//! This module provides a lock that wraps the POSIX `pthread_rwlock_t`
//! primitive provided by the macOS `libc`. It supports both exclusive (writer)
//! and shared (reader) access natively through the operating system's threading
//! library.
//!
//! # Design
//!
//! Unlike the Linux futex implementation which manages state in user-space and
//! only falls back to the kernel on contention, this implementation delegates
//! all lock management, thread parking, and waking entirely to the macOS kernel
//! via `pthread_rwlock_*` functions. This makes it simpler but potentially
//! slightly slower for uncontended locks compared to a pure user-space atomic
//! lock.

use crate::traits::{LockPolicy, SharingPolicy};
use crate::{LockResult, LockStatus};

/// A macOS `pthread_rwlock_t`-based lock and read-write lock implementation.
///
/// This struct wraps a `libc::pthread_rwlock_t` inside an
/// [`core::cell::UnsafeCell`] to allow interior mutability required by the
/// POSIX API.
#[allow(missing_debug_implementations)]
pub struct Os
{
    rwlock: core::cell::UnsafeCell<libc::pthread_rwlock_t>,
}

impl Os
{
    /// Creates a new, unlocked `Os` lock.
    ///
    /// This initializes the underlying `pthread_rwlock_t` using
    /// `pthread_rwlock_init`.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `pthread_rwlock_init` fails.
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
    /// Destroys the underlying `pthread_rwlock_t`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that no threads are currently waiting on or
    /// holding the lock when it is dropped.
    fn drop(&mut self)
    {
        unsafe {
            libc::pthread_rwlock_destroy(self.rwlock.get());
        }
    }
}

// SAFETY:
// The underlying `pthread_rwlock_t` is designed to be shared across
// threads.
unsafe impl Send for Os {}
unsafe impl Sync for Os {}

unsafe impl LockPolicy for Os
{
    type Error = core::convert::Infallible;

    /// Attempts to acquire the lock for exclusive (writer) access.
    ///
    /// This method calls `pthread_rwlock_trywrlock`. If the lock is already
    /// held, it returns [`LockStatus::Fail`] immediately without blocking.
    ///
    /// # Safety
    ///
    /// The caller must ensure proper synchronization when accessing protected
    /// data.
    unsafe fn try_lock(&self, _current_iteration: usize) -> LockResult
    {
        let result =
            unsafe { libc::pthread_rwlock_trywrlock(self.rwlock.get()) };

        if result == 0
        {
            LockResult::Ok(LockStatus::Done)
        }
        else
        {
            LockResult::Ok(LockStatus::Fail)
        }
    }

    /// Wakes all threads waiting for an exclusive lock.
    ///
    /// This is a no-op because `pthread_rwlock_t` handles thread waking
    /// automatically upon release.
    fn wake_all(&self) {}
}

unsafe impl SharingPolicy for Os
{
    /// Attempts to acquire the lock for shared (reader) access.
    ///
    /// This method calls `pthread_rwlock_tryrdlock`. If the lock is held
    /// exclusively by a writer, it returns [`LockStatus::Fail`] immediately.
    fn try_share(&self, _current_iteration: usize) -> LockResult
    {
        let result =
            unsafe { libc::pthread_rwlock_tryrdlock(self.rwlock.get()) };

        if result == 0
        {
            LockResult::Ok(LockStatus::Done)
        }
        else
        {
            LockResult::Ok(LockStatus::Fail)
        }
    }

    /// Releases a shared (reader) lock.
    fn free_share(&self)
    {
        unsafe {
            libc::pthread_rwlock_unlock(self.rwlock.get());
        }
    }

    /// Wakes all threads waiting for a shared lock.
    ///
    /// This is a no-op because `pthread_rwlock_t` handles thread waking
    /// automatically upon release.
    fn wake_readers(&self) {}
}
