use core::sync::atomic::{AtomicU32, Ordering};

use crate::{ILock, LockResult};

/// .
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Os(AtomicU32);

impl Os
{
    /// Creates a new unlocked mutex.
    pub const fn new() -> Self
    {
        Self(AtomicU32::new(0))
    }

    #[inline]
    fn futex_wait(&self, expected: u32)
    {
        let ptr = self.0.as_ptr();
        unsafe {
            libc::syscall(
                libc::SYS_futex,
                ptr,
                libc::FUTEX_WAIT,
                expected,
                std::ptr::null::<libc::timespec>(),
            );
        }
    }

    #[inline]
    fn futex_wake(&self)
    {
        let ptr = self.0.as_ptr();
        unsafe {
            libc::syscall(
                libc::SYS_futex,
                ptr,
                libc::FUTEX_WAKE,
                1, // wake one waiter
            );
        }
    }
}

unsafe impl ILock for Os
{
    fn try_lock(&self) -> LockResult
    {
        loop
        {
            if self
                .0
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return LockResult::Done;
            }
            self.futex_wait(0);
        }
    }

    fn free(&self)
    {
        // If the state is 2 (locked with waiters), we need to wake one waiter.
        let prev = self.0.swap(0, Ordering::Release);
        if prev == 2
        {
            self.futex_wake();
        }
        // If prev == 1, just unlock without waking (no waiters).
        // If prev == 0, this is a double-free, but we ignore it for safety.
    }

    fn fake_lock(&self) -> LockResult
    {
        // Fast path: try to acquire with no waiters.
        if self
            .0
            .compare_exchange(0, 0, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return LockResult::Done;
        }
        // The lock is held – we can't acquire.
        LockResult::Fail
    }
}
