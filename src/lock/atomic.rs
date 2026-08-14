//! An atomic counter‑based lock that implements both [`ILock`] and [`IShare`].

use core::convert::Infallible;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{ILock, IShare, LockResult, LockStatus};

const WRITER: usize = usize::MAX;

/// A lock that uses a single [`AtomicUsize`] as its underlying state.
#[allow(missing_debug_implementations)]
pub struct Atomic(AtomicUsize);

impl Atomic
{
    /// Creates a new unlocked [`Atomic`] lock.
    pub const fn new() -> Self
    {
        Self(AtomicUsize::new(0))
    }
}

impl core::default::Default for Atomic
{
    fn default() -> Self
    {
        Self::new()
    }
}

unsafe impl ILock for Atomic
{
    type Error = Infallible;

    fn try_lock(&self, _current_iteration: usize) -> LockResult<Self::Error>
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

    fn fake_lock(&self) -> LockResult<Self::Error>
    {
        if self.0.load(Ordering::Relaxed) == 0
        {
            Ok(LockStatus::Done)
        }
        else
        {
            Ok(LockStatus::Fail)
        }
    }

    fn free(&self)
    {
        self.0.store(0, Ordering::Release);
    }
}

impl IShare for Atomic
{
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

    fn free_share(&self)
    {
        self.0.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn atomic_new_is_unlocked()
    {
        let lock = Atomic::new();
        assert_eq!(lock.try_lock(0), Ok(LockStatus::Done));
        lock.free();
    }

    #[test]
    fn atomic_writer_blocks_writer()
    {
        let lock = Atomic::new();
        assert_eq!(lock.try_lock(0), Ok(LockStatus::Done));
        assert_eq!(lock.try_lock(0), Ok(LockStatus::Fail));
        lock.free();
        assert_eq!(lock.try_lock(0), Ok(LockStatus::Done));
        lock.free();
    }

    #[test]
    fn atomic_multiple_readers_ok()
    {
        let lock = Atomic::new();
        assert_eq!(lock.try_share(0), Ok(LockStatus::Done));
        assert_eq!(lock.try_share(0), Ok(LockStatus::Done));
        lock.free_share();
        lock.free_share();
    }
}
