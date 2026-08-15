//! An atomic-based implementation of [`LockPolicy`] and [`SharingPolicy`].
//!
//! This module provides a lock that relies on
//! [`core::sync::atomic::AtomicUsize`] for both exclusive (writer) and shared
//! (reader) access. It is fully compatible with `#![no_std]` environments and
//! does not require any operating system support.
//!
//! # Design
//!
//! The lock state is represented by a single `AtomicUsize`:
//! - `0`: The lock is completely free.
//! - `usize::MAX`: The lock is held exclusively by a writer.
//! - `1..=(usize::MAX - 1)`: The lock is held by one or more readers.
//!
//! Because it relies purely on atomic instructions, it is highly portable but
//! does not support thread parking. Threads waiting for the lock must rely on
//! a OS-driven lock.

use crate::traits::{LockPolicy, SharingPolicy};
use crate::{LockResult, LockStatus};
use core::convert::Infallible;
use core::sync::atomic::{AtomicUsize, Ordering};

/// The state value indicating that the lock is held exclusively by a writer.
const WRITER: usize = usize::MAX;

/// An atomic-based lock and read-write lock implementation.
///
/// This struct uses an [`AtomicUsize`] to track the lock state. It implements
/// both [`LockPolicy`] for exclusive access and [`SharingPolicy`] for shared
/// (reader) access.
///
/// # Examples
///
/// ```rust
/// # use resync::traits::{LockPolicy, SharingPolicy};
/// # use resync::lock::Atomic;
/// # use resync::LockStatus;
/// let lock = Atomic::new();
///
/// // Acquire exclusive access
/// assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done);
///
/// // Release exclusive access
/// unsafe { lock.free() };
///
/// // Acquire shared access
/// assert_eq!(lock.try_share(0).unwrap(), LockStatus::Done);
///
/// // Release shared access
/// lock.free_share();
/// ```
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct Atomic(AtomicUsize);

impl Atomic
{
    /// Creates a new, unlocked `Atomic` lock.
    ///
    /// This is a `const` function, allowing the lock to be initialized in
    /// static variables.
    pub const fn new() -> Self
    {
        Self(AtomicUsize::new(0))
    }
}

unsafe impl LockPolicy for Atomic
{
    type Error = Infallible;

    /// Attempts to acquire the lock for exclusive (writer) access.
    ///
    /// This method uses an atomic compare-exchange to transition the state
    /// from `0` (unlocked) to [`WRITER`] (`usize::MAX`).
    ///
    /// # Safety
    ///
    /// The caller must ensure that proper memory ordering is maintained when
    /// accessing the protected data. This implementation uses `Acquire`
    /// ordering on success to ensure visibility of prior writes.
    unsafe fn try_lock(
        &self,
        _current_iteration: usize,
    ) -> LockResult<Self::Error>
    {
        if self
            .0
            .compare_exchange(0, WRITER, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Ok(LockStatus::Done)
        }
        else
        {
            Ok(LockStatus::Fail)
        }
    }

    // /// Checks the current state of the lock without modifying it.
    // ///
    // /// # Returns
    // ///
    // /// - [`LockStatus::Done`]: The lock is currently free (`state == 0`).
    // /// - [`LockStatus::Fail`]: The lock is currently held by a writer or one
    // or ///   more readers.
    // fn get_state(&self) -> LockResult<Self::Error>
    // {
    //     if self.0.load(Ordering::Relaxed) == 0
    //     {
    //         Ok(LockStatus::Done)
    //     }
    //     else
    //     {
    //         Ok(LockStatus::Fail)
    //     }
    // }

    /// Releases the exclusive (writer) lock.
    ///
    /// This method resets the state to `0` using `Release` ordering, ensuring
    /// that all writes performed while the lock was held are visible to the
    /// next thread that acquires the lock.
    ///
    /// # Safety
    ///
    /// The caller must ensure that they currently hold the exclusive lock.
    /// Calling this method when the lock is not held may corrupt the state
    /// and allow concurrent access to the protected data.
    unsafe fn free(&self)
    {
        self.0.store(0, Ordering::Release);
    }
}

unsafe impl SharingPolicy for Atomic
{
    /// Attempts to acquire the lock for shared (reader) access.
    ///
    /// This method increments the reader count atomically, provided that no
    /// writer currently holds the lock (i.e., the state is not [`WRITER`]).
    /// If a writer holds the lock, it returns [`LockStatus::Fail`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that proper memory ordering is maintained.
    /// This implementation uses `Acquire` ordering on the successful
    /// compare-exchange to ensure visibility of prior writes.
    fn try_share(&self, _current_iteration: usize) -> LockResult<Self::Error>
    {
        loop
        {
            let state = self.0.load(Ordering::Relaxed);
            if state == WRITER
            {
                return Ok(LockStatus::Fail);
            }

            if self
                .0
                .compare_exchange_weak(
                    state,
                    state + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Ok(LockStatus::Done);
            }
        }
    }

    /// Releases a shared (reader) lock.
    ///
    /// This method decrements the reader count atomically using `Release`
    /// ordering.
    ///
    /// # Safety
    ///
    /// The caller must ensure that they currently hold a shared lock. Calling
    /// this method without holding a shared lock will corrupt the reader count.
    fn free_share(&self)
    {
        self.0.fetch_sub(1, Ordering::Release);
    }
}
