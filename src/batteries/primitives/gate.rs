//! A gate or valve synchronization primitive.
//!
//! This module provides the [`Gate`] struct, which acts as a controllable
//! barrier for multiple threads. It leverages a [`SharingPolicy`] to implement
//! its semantics:
//! - **Closed**: The underlying lock is held exclusively (Writer). Threads
//!   calling [`wait`](Self::wait) will block until the gate is opened.
//! - **Open**: The underlying lock is free. Threads calling
//!   [`wait`](Self::wait) can pass through concurrently by acquiring and
//!   immediately releasing a shared (Reader) lock.
//!
//! By default, a newly created `Gate` is in the **closed** state to prevent
//! threads from passing through before the barrier is explicitly opened.

use crate::traits::{NewLocked, RetryPolicy, SharingPolicy};
use crate::{AcquireError, LockStatus, TryLockError};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

/// A synchronization primitive that acts as a controllable gate or valve.
///
/// A `Gate` can be in one of two states:
/// - **Open**: Threads calling [`wait`](Self::wait) or
///   [`try_wait`](Self::try_wait) will pass through immediately.
/// - **Closed**: Threads calling [`wait`](Self::wait) will block until the gate
///   is opened. [`try_wait`](Self::try_wait) will return an error.
///
/// # Examples
///
/// ```rust
/// # use resync::{Gate, lock::Atomic, retry::Yield};
/// # use std::sync::Arc;
/// # use std::thread;
/// # use std::time::Duration;
/// // Created in the closed state by default
/// let gate = Arc::new(Gate::<Atomic, Yield>::new());
///
/// let g2 = Arc::clone(&gate);
/// let handle = thread::spawn(move || {
///     // This will block until the gate is opened
///     g2.wait().unwrap();
///     println!("Thread passed the gate!");
/// });
///
/// thread::sleep(Duration::from_millis(100));
/// gate.open(); // Unblocks the waiting thread
///
/// handle.join().unwrap();
/// ```
#[allow(missing_debug_implementations)]
pub struct Gate<L, R>
where
    L: SharingPolicy,
    R: RetryPolicy,
{
    inner:     L,
    retry:     R,
    /// Stores the writer lock metadata when the gate is closed.
    meta:      UnsafeCell<Option<L::Meta>>,
    /// Fast-path flag to check if the gate is closed without touching the
    /// lock.
    is_closed: AtomicBool,
}

// SAFETY:
// The gate manages synchronization state. It is safe to send across threads
// if the underlying policies and metadata are Send.
unsafe impl<L, R> core::marker::Send for Gate<L, R>
where
    L: SharingPolicy + core::marker::Send,
    R: RetryPolicy + core::marker::Send,
    L::Meta: core::marker::Send,
{
}

// SAFETY:
// The gate is safe to share across threads. Concurrent access to `meta` is
// strictly guarded by the `is_closed` atomic flag and the underlying
// `SharingPolicy` (which ensures exclusive access when modifying `meta`).
unsafe impl<L, R> core::marker::Sync for Gate<L, R>
where
    L: SharingPolicy + core::marker::Sync,
    R: RetryPolicy + core::marker::Sync,
    L::Meta: core::marker::Sync,
{
}

impl<L, R> core::fmt::Debug for Gate<L, R>
where
    L: SharingPolicy,
    R: RetryPolicy,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.write_str("Gate { ... }")
    }
}

impl<L, R> Gate<L, R>
where
    L: SharingPolicy + NewLocked,
    R: RetryPolicy + Default,
{
    /// Creates a new `Gate` in the **closed** (locked) state.
    ///
    /// Threads calling [`wait`](Self::wait) on a newly created gate will
    /// block until [`open`](Self::open) is called.
    pub fn new() -> Self
    {
        let (meta, inner) = L::new_locked();
        Self {
            inner,
            retry: R::default(),
            meta: UnsafeCell::new(Some(meta)),
            is_closed: AtomicBool::new(true),
        }
    }
}

impl<L, R> core::default::Default for Gate<L, R>
where
    L: SharingPolicy + NewLocked,
    R: RetryPolicy + Default,
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl<L, R> Gate<L, R>
where
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
{
    /// Creates a new `Gate` in the **open** (unlocked) state.
    ///
    /// Threads calling [`wait`](Self::wait) on a newly created gate will
    /// pass through immediately. Call [`close`](Self::close) to start
    /// blocking incoming threads.
    pub fn new_open() -> Self
    {
        Self {
            inner:     L::default(),
            retry:     R::default(),
            meta:      UnsafeCell::new(None),
            is_closed: AtomicBool::new(false),
        }
    }
}

impl<L, R> Gate<L, R>
where
    L: SharingPolicy,
    R: RetryPolicy,
{
    /// Closes the gate.
    ///
    /// This acquires the underlying exclusive (Writer) lock. Any subsequent
    /// calls to [`wait`](Self::wait) will block until [`open`](Self::open)
    /// is called.
    ///
    /// If the gate is already closed, this method returns immediately.
    pub fn close(&self) -> Result<(), AcquireError<(), L::Error, R::Error>>
    {
        if self.is_closed.load(Ordering::Acquire)
        {
            return Ok(());
        }

        let mut iterations = 0usize;
        loop
        {
            iterations += 1;
            match unsafe { self.inner.try_lock(iterations) }
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    // SAFETY: We just acquired the exclusive writer lock.
                    // No readers can be holding the lock, and no other thread
                    // can be modifying `meta` concurrently.
                    unsafe {
                        *self.meta.get() = Some(meta);
                    }
                    self.is_closed.store(true, Ordering::Release);
                    return Ok(());
                },
                Ok(LockStatus::Fail) =>
                {
                    if let Err(e) = self.retry.retry(iterations)
                    {
                        return Err(AcquireError::Retry(e));
                    }
                },
                Err(e) => return Err(AcquireError::Lock(e)),
            }
        }
    }

    /// Opens the gate.
    ///
    /// This releases the underlying exclusive (Writer) lock, allowing all
    /// threads currently blocked in [`wait`](Self::wait) to proceed.
    ///
    /// If the gate is already open, this method is a no-op.
    pub fn open(&self)
    {
        if !self.is_closed.load(Ordering::Acquire)
        {
            return;
        }

        // SAFETY: We only access `meta` if `is_closed` is true.
        // We take the metadata out to ensure we only call `free` once per
        // close/open cycle, even if `open` is called concurrently by
        // multiple threads.
        let meta_opt = unsafe { (*self.meta.get()).take() };
        if let Some(meta) = meta_opt
        {
            unsafe { self.inner.free(&meta) };
            self.is_closed.store(false, Ordering::Release);
        }
    }

    /// Waits for the gate to open.
    ///
    /// If the gate is currently closed, this method will block (using the
    /// configured [`RetryPolicy`]) until another thread calls
    /// [`open`](Self::open).
    ///
    /// Once the gate is open, this method acquires a shared (Reader) lock
    /// and immediately releases it, allowing the thread to pass through.
    pub fn wait(&self) -> Result<(), AcquireError<(), L::Error, R::Error>>
    {
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;
            match self.inner.try_share(iterations)
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    // We successfully passed the gate. Release the reader lock
                    // immediately.
                    self.inner.free_share(&meta);
                    return Ok(());
                },
                Ok(LockStatus::Fail) =>
                {
                    if let Err(e) = self.retry.retry(iterations)
                    {
                        return Err(AcquireError::Retry(e));
                    }
                },
                Err(e) => return Err(AcquireError::Lock(e)),
            }
        }
    }

    /// Attempts to pass through the gate without blocking.
    ///
    /// If the gate is closed, this method returns
    /// `Err(TryLockError::Contention)`. If the gate is open, it acquires
    /// and immediately releases a shared lock, returning `Ok(())`.
    pub fn try_wait(&self) -> Result<(), TryLockError<(), L::Error>>
    {
        match self.inner.try_share(0)
        {
            Ok(LockStatus::Done(meta)) =>
            {
                self.inner.free_share(&meta);
                Ok(())
            },
            Ok(LockStatus::Fail) => Err(TryLockError::Contention),
            Err(e) => Err(TryLockError::Lock(e)),
        }
    }
}
