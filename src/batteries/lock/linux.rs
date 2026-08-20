//! A Linux-specific implementation of [`LockPolicy`] and [`SharingPolicy`]
//! using futexes.
//!
//! This module provides a highly efficient lock that uses an [`AtomicU32`] for
//! fast-path user-space acquisition and falls back to the Linux `futex` system
//! call for parking and waking threads when contention occurs.
//!
//! # Design
//!
//! The lock state is packed into a single `u32`:
//! - **Bit 31 (`WRITER`)**: Set if the lock is held exclusively by a writer.
//! - **Bit 30 (`WAITERS`)**: Set if there are threads parked (waiting) on the
//!   lock.
//! - **Bits 0–29 (`READERS_MASK`)**: The count of concurrent readers.
//!
//! This design allows the lock to support both exclusive (writer) and shared
//! (reader) access while minimizing system call overhead. Threads only enter
//! the kernel via `futex_wait` when they fail to acquire the lock after a
//! certain number of spin iterations (`DEFAULT_EPSILON`).
use crate::traits::{LockPolicy, NewLocked, SharingPolicy};
use crate::{LockResult, LockStatus};
use core::convert::Infallible;
use core::sync::atomic::{AtomicU32, Ordering};

/// The number of fast-path iterations before falling back to the kernel futex.
const DEFAULT_EPSILON: usize = 10000;

/// Bitmask indicating that the lock is held exclusively by a writer.
const WRITER: u32 = 1 << 31;

/// Bitmask indicating that there are threads waiting (parked) on the lock.
const WAITERS: u32 = 1 << 30;

/// Bitmask for extracting the reader count from the lock state.
const READERS_MASK: u32 = !(WRITER | WAITERS);

/// A Linux futex-based lock and read-write lock implementation.
///
/// This struct provides high-performance synchronization on Linux by combining
/// user-space atomic operations with kernel-space thread parking via the
/// `futex` system call.
///
/// It implements both [`LockPolicy`] and [`SharingPolicy`], making it suitable
/// for use as a standard mutex or a read-write lock.
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Futex(AtomicU32);

impl Futex
{
    /// Creates a new, unlocked `Os` lock.
    ///
    /// This is a `const` function, allowing the lock to be initialized in
    /// static variables.
    pub const fn new() -> Self
    {
        Self(AtomicU32::new(0))
    }

    /// The slow path for acquiring an exclusive (writer) lock.
    ///
    /// This method is called when the fast-path atomic exchange fails
    /// repeatedly. It sets the `WAITERS` flag to indicate that threads are
    /// parking, and then invokes `futex_wait` to put the current thread to
    /// sleep until the lock is released.
    fn lock_slow(&self) -> LockResult<(), Infallible>
    {
        loop
        {
            let state = self.0.load(Ordering::Relaxed);
            if state == 0
            {
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
                    return LockResult::Ok(LockStatus::Done(()));
                }
                continue;
            }
            if state & WAITERS != 0
            {
                futex_wait(&self.0, state);
                continue;
            }
            if self
                .0
                .compare_exchange_weak(
                    state,
                    state | WAITERS,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                futex_wait(&self.0, state | WAITERS);
            }
        }
    }

    /// The slow path for acquiring a shared (reader) lock.
    ///
    /// Similar to [`lock_slow`], this method parks the current thread using
    /// `futex_wait` if the lock is held by a writer or if there are already
    /// waiters (to prevent reader starvation of writers).
    fn share_slow(&self) -> LockResult<(), Infallible>
    {
        loop
        {
            let state = self.0.load(Ordering::Relaxed);
            if state & WRITER != 0
            {
                if state & WAITERS != 0
                {
                    futex_wait(&self.0, state);
                }
                else if self
                    .0
                    .compare_exchange_weak(
                        state,
                        state | WAITERS,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    futex_wait(&self.0, state | WAITERS);
                }
                continue;
            }
            if state & WAITERS != 0
            {
                futex_wait(&self.0, state);
                continue;
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
                return LockResult::Ok(LockStatus::Done(()));
            }
        }
    }
}

unsafe impl LockPolicy for Futex
{
    type Error = core::convert::Infallible;
    type Meta = ();

    /// Attempts to acquire the lock for exclusive (writer) access.
    ///
    /// This method first tries a fast-path atomic compare-exchange. If it
    /// fails and the `current_iteration` exceeds [`DEFAULT_EPSILON`], it falls
    /// back to the kernel-space [`lock_slow`] path using futexes.
    ///
    /// # Safety
    ///
    /// The caller must ensure proper memory ordering when accessing protected
    /// data.
    unsafe fn try_lock(
        &self,
        current_iteration: usize,
    ) -> LockResult<Self::Meta, Infallible>
    {
        match self.0.compare_exchange(
            0,
            WRITER,
            Ordering::Acquire,
            Ordering::Relaxed,
        )
        {
            Ok(_) => LockResult::Ok(LockStatus::Done(())),
            Err(_) if current_iteration >= DEFAULT_EPSILON => self.lock_slow(),
            Err(_) => LockResult::Ok(LockStatus::Fail),
        }
    }

    /// Releases the exclusive (writer) lock.
    ///
    /// If there are threads waiting on the lock (indicated by the `WAITERS`
    /// flag), this method wakes them up using `futex_wake`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that they currently hold the exclusive lock.
    unsafe fn free(&self, _: &Self::Meta)
    {
        let old = self.0.swap(0, Ordering::Release);
        if old & WAITERS != 0
        {
            futex_wake(&self.0, i32::MAX);
        }
    }

    /// Wakes all threads waiting for an exclusive lock.
    fn wake_all(&self)
    {
        if self.0.load(Ordering::Relaxed) & WAITERS != 0
        {
            futex_wake(&self.0, i32::MAX);
        }
    }
}

impl NewLocked for Futex
{
    /// Creates a new `Os` lock that is already acquired for exclusive access.
    ///
    /// The underlying atomic state is initialized with the `WRITER` flag set.
    /// Note that since this bypasses the fast-path CAS, no `WAITERS` flag is
    /// set initially. Any subsequent `try_lock` calls from other threads will
    /// observe the `WRITER` flag and proceed to the slow path (futex parking).
    fn new_locked() -> (Self::Meta, Self)
    {
        ((), Self(AtomicU32::new(WRITER)))
    }
}

unsafe impl SharingPolicy for Futex
{
    /// Attempts to acquire the lock for shared (reader) access.
    ///
    /// This method tries to increment the reader count. If a writer holds the
    /// lock, or if there are waiters (to maintain writer preference), it may
    /// fall back to the [`share_slow`] path.
    fn try_share(
        &self,
        current_iteration: usize,
    ) -> LockResult<Self::Meta, Infallible>
    {
        loop
        {
            let state = self.0.load(Ordering::Relaxed);

            // Writer holds the lock -> fail or slow path
            if state & WRITER != 0
            {
                return if current_iteration >= DEFAULT_EPSILON
                {
                    self.share_slow()
                }
                else
                {
                    LockResult::Ok(LockStatus::Fail)
                };
            }

            // Writer‑preferred: if anyone is waiting, don't let new readers in
            if state & WAITERS != 0
            {
                return if current_iteration >= DEFAULT_EPSILON
                {
                    self.share_slow()
                }
                else
                {
                    LockResult::Ok(LockStatus::Fail)
                };
            }

            // Try to increment reader count
            match self.0.compare_exchange_weak(
                state,
                state + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            {
                Ok(_) => return LockResult::Ok(LockStatus::Done(())),
                Err(_) => continue,
            }
        }
    }

    /// Releases a shared (reader) lock.
    ///
    /// If this was the last reader and there are threads waiting, it wakes
    /// one waiter (typically a writer).
    fn free_share(&self, _: &Self::Meta)
    {
        let old = self.0.fetch_sub(1, Ordering::Release);
        let readers = old & READERS_MASK;

        // We were the last reader and someone is waiting
        if readers == 1 && old & WAITERS != 0
        {
            futex_wake(&self.0, 1);
        }
    }

    /// Wakes all threads waiting for a shared (reader) lock.
    fn wake_readers(&self)
    {
        if self.0.load(Ordering::Relaxed) & WAITERS != 0
        {
            futex_wake(&self.0, i32::MAX);
        }
    }
}

/// Linux `futex` operation constants.
const FUTEX_WAIT: i32 = 0;
const FUTEX_WAKE: i32 = 1;
const FUTEX_PRIVATE_FLAG: i32 = 128;

/// Invokes the Linux `futex` system call to put the current thread to sleep.
///
/// The thread will sleep until the value at `atomic` changes from `expected`,
/// or until it is woken by a signal or another thread calling [`futex_wake`].
///
/// # Safety
///
/// This function performs a raw system call. The caller must ensure that
/// `atomic` points to valid, aligned memory that is safe to pass to the kernel.
#[inline]
fn futex_wait(atomic: &AtomicU32, expected: u32)
{
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            atomic.as_ptr(),
            FUTEX_WAIT | FUTEX_PRIVATE_FLAG,
            expected,
            core::ptr::null::<libc::timespec>(),
        );
    }
}

/// Invokes the Linux `futex` system call to wake sleeping threads.
///
/// Wakes up to `count` threads waiting on the futex at `atomic`.
///
/// # Safety
///
/// This function performs a raw system call. The caller must ensure that
/// `atomic` points to valid, aligned memory.
#[inline]
fn futex_wake(atomic: &AtomicU32, count: i32)
{
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            atomic.as_ptr(),
            FUTEX_WAKE | FUTEX_PRIVATE_FLAG,
            count,
        );
    }
}
