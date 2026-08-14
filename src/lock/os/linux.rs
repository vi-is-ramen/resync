//! A futex‑based reader‑writer lock for Linux.
//!
//! This single type implements both [`ILock`] (exclusive/writer access) and
//! [`IShare`] (shared/reader access), using one [`AtomicU32`] as both the
//! lock state and the futex address.
//!
//! # State Encoding (u32)
//!
//! ```text
//! Bit 31:       WRITER   — exclusive lock is held
//! Bit 30:       WAITERS  — at least one thread is sleeping on the futex
//! Bits 0–29:    READERS  — number of active readers (0 .. 2^30-1)
//! ```
//!
//! The lock is **writer‑preferred**: when a writer is waiting (WAITERS bit
//! set), new readers are blocked until the writer has had a chance to
//! acquire the lock. This prevents writer starvation.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::{DEFAULT_EPSILON, ILock, IShare, LockResult, LockStatus};

/// Bit 31: writer holds the lock.
const WRITER: u32 = 1 << 31;

/// Bit 30: at least one thread is parked on the futex.
const WAITERS: u32 = 1 << 30;

/// Mask for bits 0–29: reader count.
const READERS_MASK: u32 = !(WRITER | WAITERS);

/// Futex‑based reader‑writer lock for Linux.
///
/// Implements both [`ILock`] (writer access) and [`IShare`] (reader access).
/// When `current_iteration >= DEFAULT_EPSILON`, the lock parks the current
/// thread via `FUTEX_WAIT` instead of spinning.
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Os(AtomicU32);

impl Os
{
    /// Creates a new unlocked [`Os`] lock.
    pub const fn new() -> Self
    {
        Self(AtomicU32::new(0))
    }

    /// Slow path for writer acquisition.
    ///
    /// Sets the `WAITERS` bit and parks the current thread via futex.
    /// After waking, retries the fast path.
    fn lock_slow(&self) -> LockResult
    {
        loop
        {
            let state = self.0.load(Ordering::Relaxed);

            // If lock is free, try fast path
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
                    return LockResult::Ok(LockStatus::Done);
                }
                continue;
            }

            // If WAITERS is already set, just sleep
            if state & WAITERS != 0
            {
                futex_wait(&self.0, state);
                continue;
            }

            // Try to set WAITERS bit
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

    /// Slow path for reader acquisition.
    ///
    /// Sets the `WAITERS` bit and parks the current thread via futex.
    /// After waking, retries the fast path.
    fn share_slow(&self) -> LockResult
    {
        loop
        {
            let state = self.0.load(Ordering::Relaxed);

            // Writer holds the lock — spin or re‑park
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

            // WAITERS set (writer‑preferred): keep waiting
            if state & WAITERS != 0
            {
                futex_wait(&self.0, state);
                continue;
            }

            // Try to increment reader count
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
                return LockResult::Ok(LockStatus::Done);
            }
        }
    }
}

unsafe impl ILock for Os
{
    type Error = core::convert::Infallible;

    /// Attempts to acquire an exclusive (writer) lock.
    ///
    /// **Fast path**: if the lock is completely free, a single CAS from `0`
    /// to `WRITER` succeeds with no system call.
    ///
    /// **Slow path**: when `current_iteration >= DEFAULT_EPSILON`, the lock
    /// sets the `WAITERS` bit and parks via `FUTEX_WAIT`.
    fn try_lock(&self, current_iteration: usize) -> LockResult
    {
        match self.0.compare_exchange(
            0,
            WRITER,
            Ordering::Acquire,
            Ordering::Relaxed,
        )
        {
            Ok(_) => LockResult::Ok(LockStatus::Done),
            Err(_) if current_iteration >= DEFAULT_EPSILON => self.lock_slow(),
            Err(_) => LockResult::Ok(LockStatus::Fail),
        }
    }

    fn fake_lock(&self) -> LockResult
    {
        if self.0.load(Ordering::Relaxed) == 0
        {
            LockResult::Ok(LockStatus::Done)
        }
        else
        {
            LockResult::Ok(LockStatus::Fail)
        }
    }

    /// Releases an exclusive (writer) lock.
    ///
    /// Atomically sets the state to `0`. If there were any waiters, wakes
    /// **all** of them — both sleeping readers and writers. The woken
    /// threads will re‑contend for the lock.
    fn free(&self)
    {
        let old = self.0.swap(0, Ordering::Release);
        if old & WAITERS != 0
        {
            futex_wake(&self.0, i32::MAX);
        }
    }

    /// Wakes all threads waiting on this lock.
    fn wake_all(&self)
    {
        if self.0.load(Ordering::Relaxed) & WAITERS != 0
        {
            futex_wake(&self.0, i32::MAX);
        }
    }
}

impl IShare for Os
{
    /// Attempts to acquire a shared (reader) lock.
    ///
    /// **Fast path**: succeeds if no writer holds the lock and no writer is
    /// waiting (writer‑preferred policy). Increments the reader count.
    ///
    /// **Slow path**: when `current_iteration >= DEFAULT_EPSILON`, the lock
    /// sets the `WAITERS` bit and parks via `FUTEX_WAIT`.
    fn try_share(&self, current_iteration: usize) -> LockResult
    {
        loop
        {
            let state = self.0.load(Ordering::Relaxed);

            // Writer holds the lock → fail or slow path
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
                Ok(_) => return LockResult::Ok(LockStatus::Done),
                Err(_) => continue,
            }
        }
    }

    /// Releases a shared (reader) lock.
    ///
    /// Decrements the reader count. If this was the **last** reader and
    /// there are waiters (writers waiting), wakes exactly one waiter.
    fn free_share(&self)
    {
        let old = self.0.fetch_sub(1, Ordering::Release);
        let readers = old & READERS_MASK;

        // We were the last reader and someone is waiting
        if readers == 1 && old & WAITERS != 0
        {
            futex_wake(&self.0, 1);
        }
    }

    /// Wakes all threads waiting for a reader lock.
    fn wake_readers(&self)
    {
        if self.0.load(Ordering::Relaxed) & WAITERS != 0
        {
            futex_wake(&self.0, i32::MAX);
        }
    }
}

const FUTEX_WAIT: i32 = 0;
const FUTEX_WAKE: i32 = 1;
const FUTEX_PRIVATE_FLAG: i32 = 128;

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

#[cfg(test)]
mod tests
{
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn os_writer_acquires()
    {
        let lock = Os::new();
        assert_eq!(lock.try_lock(0), LockResult::Ok(LockStatus::Done));
        assert_eq!(lock.try_lock(0), LockResult::Ok(LockStatus::Fail));
        lock.free();
        assert_eq!(lock.try_lock(0), LockResult::Ok(LockStatus::Done));
        lock.free();
    }

    #[test]
    fn os_multiple_readers()
    {
        let lock = Os::new();
        assert_eq!(lock.try_share(0), LockResult::Ok(LockStatus::Done));
        assert_eq!(lock.try_share(0), LockResult::Ok(LockStatus::Done));
        assert_eq!(lock.try_share(0), LockResult::Ok(LockStatus::Done));
        lock.free_share();
        lock.free_share();
        lock.free_share();
    }

    #[test]
    fn os_writer_blocks_readers()
    {
        let lock = Os::new();
        assert_eq!(lock.try_lock(0), LockResult::Ok(LockStatus::Done));
        assert_eq!(lock.try_share(0), LockResult::Ok(LockStatus::Fail));
        lock.free();
        assert_eq!(lock.try_share(0), LockResult::Ok(LockStatus::Done));
        lock.free_share();
    }

    #[test]
    fn os_readers_block_writer()
    {
        let lock = Os::new();
        assert_eq!(lock.try_share(0), LockResult::Ok(LockStatus::Done));
        assert_eq!(lock.try_lock(0), LockResult::Ok(LockStatus::Fail));
        lock.free_share();
        assert_eq!(lock.try_lock(0), LockResult::Ok(LockStatus::Done));
        lock.free();
    }

    #[test]
    fn os_concurrent_writer_and_readers()
    {
        let lock = Arc::new(Os::new());
        let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));

        let writer_lock = lock.clone();
        let writer_counter = counter.clone();
        let writer = thread::spawn(move || {
            // Acquire writer lock
            loop
            {
                if writer_lock.try_lock(0) == LockResult::Ok(LockStatus::Done)
                {
                    break;
                }
                std::hint::spin_loop();
            }
            writer_counter.fetch_add(100, Ordering::Relaxed);
            writer_lock.free();
        });

        let reader_lock = lock.clone();
        let reader_counter = counter.clone();
        let reader = thread::spawn(move || {
            loop
            {
                if reader_lock.try_share(0) == LockResult::Ok(LockStatus::Done)
                {
                    break;
                }
                std::hint::spin_loop();
            }
            let _ = reader_counter.load(Ordering::Relaxed);
            reader_lock.free_share();
        });

        writer.join().unwrap();
        reader.join().unwrap();
    }

    #[test]
    fn os_free_is_idempotent()
    {
        let lock = Os::new();
        lock.free();
        assert_eq!(lock.try_lock(0), LockResult::Ok(LockStatus::Done));
        lock.free();
        lock.free();
        assert_eq!(lock.try_lock(0), LockResult::Ok(LockStatus::Done));
        lock.free();
    }
}
