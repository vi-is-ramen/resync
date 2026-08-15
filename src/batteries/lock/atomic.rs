use crate::traits::{LockPolicy, SharingPolicy};
use crate::{LockResult, LockStatus};
use core::convert::Infallible;
use core::sync::atomic::{AtomicUsize, Ordering};

const WRITER: usize = usize::MAX;

/// .
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct Atomic(AtomicUsize);

impl Atomic
{
    /// .
    pub const fn new() -> Self
    {
        Self(AtomicUsize::new(0))
    }
}

unsafe impl LockPolicy for Atomic
{
    type Error = Infallible;

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

    fn get_state(&self) -> LockResult<Self::Error>
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

    unsafe fn free(&self)
    {
        self.0.store(0, Ordering::Release);
    }
}

unsafe impl SharingPolicy for Atomic
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
