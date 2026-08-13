//! An atomic counter‑based reader‑writer lock.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{IShare, LockResult};

/// The sentinel value that indicates the lock is held by a writer.
const WRITER: usize = usize::MAX;

/// A reader‑writer lock that uses a single `AtomicUsize` as its state.
///
/// # State Encoding
/// - `0`              : the lock is free.
/// - `1..=WRITER-1`   : the lock is held by that many readers.
/// - `WRITER`         : the lock is held by a writer.
#[repr(transparent)]
#[derive(Default, Debug)]
pub struct Atomic(AtomicUsize);

impl IShare for Atomic
{
    /// Attempts to acquire a read lock.
    ///
    /// Succeeds if no writer holds the lock. Increments the reader count.
    ///
    /// # Memory Ordering
    /// - On success: [`Ordering::Acquire`] to ensure subsequent reads are
    ///   ordered after the lock acquisition.
    /// - On failure: [`Ordering::Relaxed`].
    ///
    /// # Returns
    /// - [`LockResult::Done`] – read lock acquired.
    /// - [`LockResult::Fail`] – a writer currently holds the lock.
    fn try_read(&self) -> LockResult
    {
        let state = self.0.load(Ordering::Relaxed);
        if state == WRITER
        {
            return LockResult::Fail;
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
            LockResult::Done
        }
        else
        {
            LockResult::Fail
        }
    }

    /// Attempts to acquire a write lock.
    ///
    /// Succeeds only if the lock is currently free (`counter == 0`).
    /// Sets the counter to `WRITER`.
    ///
    /// # Memory Ordering
    /// - On success: [`Ordering::Acquire`].
    /// - On failure: [`Ordering::Relaxed`].
    ///
    /// # Returns
    /// - [`LockResult::Done`] – write lock acquired.
    /// - [`LockResult::Fail`] – the lock is currently held (by readers or a
    ///   writer).
    fn try_write(&self) -> LockResult
    {
        let state = self.0.load(Ordering::Relaxed);
        if state != 0
        {
            return LockResult::Fail;
        }

        if self
            .0
            .compare_exchange_weak(
                0,
                WRITER,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            LockResult::Done
        }
        else
        {
            LockResult::Fail
        }
    }

    /// Releases a read lock by decrementing the reader count.
    ///
    /// # Memory Ordering
    /// Uses [`Ordering::Release`] to ensure all previous reads are
    /// completed before the release.
    ///
    /// # Safety
    /// Must only be called when a read lock is held.
    fn free_read(&self)
    {
        self.0.fetch_sub(1, Ordering::Release);
    }

    /// Releases a write lock by resetting the counter to `0`.
    ///
    /// # Memory Ordering
    /// Uses [`Ordering::Release`].
    ///
    /// # Safety
    /// Must only be called when the write lock is held.
    fn free_write(&self)
    {
        self.0.store(0, Ordering::Release);
    }
}
