use core::sync::atomic::{AtomicU32, Ordering};

use crate::traits::{LockPolicy, SharingPolicy};
use crate::{LockResult, LockStatus};

// TODO: make Epsilon generic parameter of Os
const DEFAULT_EPSILON: usize = 10000;

const WRITER: u32 = 1 << 31;

const WAITERS: u32 = 1 << 30;

const READERS_MASK: u32 = !(WRITER | WAITERS);

/// .
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Os(AtomicU32);

impl Os
{
    /// .
    pub const fn new() -> Self
    {
        Self(AtomicU32::new(0))
    }

    fn lock_slow(&self) -> LockResult
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
                    return LockResult::Ok(LockStatus::Done);
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

    fn share_slow(&self) -> LockResult
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
                return LockResult::Ok(LockStatus::Done);
            }
        }
    }
}

unsafe impl LockPolicy for Os
{
    type Error = core::convert::Infallible;

    unsafe fn try_lock(&self, current_iteration: usize) -> LockResult
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

    fn get_state(&self) -> LockResult
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

    unsafe fn free(&self)
    {
        let old = self.0.swap(0, Ordering::Release);
        if old & WAITERS != 0
        {
            futex_wake(&self.0, i32::MAX);
        }
    }

    fn wake_all(&self)
    {
        if self.0.load(Ordering::Relaxed) & WAITERS != 0
        {
            futex_wake(&self.0, i32::MAX);
        }
    }
}

unsafe impl SharingPolicy for Os
{
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
