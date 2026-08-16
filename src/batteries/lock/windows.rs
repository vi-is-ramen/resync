//! A Windows-specific implementation of [`LockPolicy`] and [`SharingPolicy`]
//! using Slim Reader/Writer (SRW) locks.
//!
//! This module provides a lock that wraps the Windows `SRWLOCK` primitive.
//! SRW locks are highly efficient, kernel-managed synchronization primitives
//! that support both exclusive (writer) and shared (reader) access.
//!
//! # Design
//!
//! Unlike futexes or user-space spinlocks, `SRWLOCK` does not require explicit
//! initialization or destruction, making it very lightweight in terms of memory
//! and lifecycle management. It also handles thread parking and waking
//! automatically within the Windows kernel.
use crate::traits::{LockPolicy, NewLocked, SharingPolicy};
use crate::{LockResult, LockStatus};

/// A Windows SRW lock-based implementation of a read-write lock.
///
/// This struct wraps a `windows_sys::Win32::System::Threading::SRWLOCK`.
#[allow(missing_debug_implementations)]
#[repr(transparent)]
pub struct Os
{
    srwlock: windows_sys::Win32::System::Threading::SRWLOCK,
}

impl Os
{
    /// Creates a new, unlocked `Os` lock.
    ///
    /// `SRWLOCK` does not require explicit initialization, so this simply
    /// sets the internal pointer to null.
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

// SAFETY:
// SRWLOCK is designed to be shared across threads.
unsafe impl Send for Os {}
unsafe impl Sync for Os {}

unsafe impl LockPolicy for Os
{
    type Error = core::convert::Infallible;
    type Meta = ();

    /// Attempts to acquire the lock for exclusive (writer) access.
    ///
    /// This method calls `TryAcquireSRWLockExclusive`. If the lock is already
    /// held, it returns [`LockStatus::Fail`] immediately without blocking.
    ///
    /// # Safety
    ///
    /// The caller must ensure proper synchronization when accessing protected
    /// data.
    unsafe fn try_lock(
        &self,
        _current_iteration: usize,
    ) -> LockResult<Self::Meta, Self::Error>
    {
        let result = unsafe {
            windows_sys::Win32::System::Threading::TryAcquireSRWLockExclusive(
                &self.srwlock as *const _ as *mut _,
            )
        };

        if result
        {
            LockResult::Ok(LockStatus::Done(()))
        }
        else
        {
            LockResult::Ok(LockStatus::Fail)
        }
    }

    /// Releases the exclusive (writer) lock.
    ///
    /// # Safety
    ///
    /// The caller must ensure that they currently hold the exclusive lock.
    unsafe fn free(&self, _: &Self::Meta)
    {
        unsafe {
            windows_sys::Win32::System::Threading::ReleaseSRWLockExclusive(
                &self.srwlock as *const _ as *mut _,
            );
        }
    }

    /// Wakes all threads waiting for an exclusive lock.
    ///
    /// This is a no-op because `SRWLOCK` handles thread waking automatically.
    fn wake_all(&self) {}
}

impl NewLocked for Os
{
    /// Creates a new `Os` lock and immediately acquires it for exclusive
    /// (writer) access using a blocking `AcquireSRWLockExclusive` call.
    fn new_locked() -> (Self::Meta, Self)
    {
        let s = Self::new();
        unsafe {
            windows_sys::Win32::System::Threading::AcquireSRWLockExclusive(
                &s.srwlock as *const _ as *mut _,
            );
        }
        ((), s)
    }
}

unsafe impl SharingPolicy for Os
{
    /// Attempts to acquire the lock for shared (reader) access.
    ///
    /// This method calls `TryAcquireSRWLockShared`. If the lock is held
    /// exclusively by a writer, it returns [`LockStatus::Fail`] immediately.
    fn try_share(
        &self,
        _current_iteration: usize,
    ) -> LockResult<Self::Meta, Self::Error>
    {
        let result = unsafe {
            windows_sys::Win32::System::Threading::TryAcquireSRWLockShared(
                &self.srwlock as *const _ as *mut _,
            )
        };

        if result
        {
            LockResult::Ok(LockStatus::Done(()))
        }
        else
        {
            LockResult::Ok(LockStatus::Fail)
        }
    }

    /// Releases a shared (reader) lock.
    fn free_share(&self, _: &Self::Meta)
    {
        unsafe {
            windows_sys::Win32::System::Threading::ReleaseSRWLockShared(
                &self.srwlock as *const _ as *mut _,
            );
        }
    }

    /// Wakes all threads waiting for a shared lock.
    ///
    /// This is a no-op because `SRWLOCK` handles thread waking automatically.
    fn wake_readers(&self) {}
}
