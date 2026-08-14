//! A composite lock that acquires two inner locks in a fixed order.

use crate::{ILock, LockResult, LockStatus};

/// A lock composed of two inner locks.
#[allow(missing_debug_implementations)]
pub struct Nested<L1: ILock, L2: ILock>
{
    l1: L1,
    l2: L2,
}

/// .
#[allow(missing_debug_implementations)]
pub enum Error<L1: ILock, L2: ILock>
{
    /// .
    Le1(<L1 as ILock>::Error),

    /// .
    Le2(<L2 as ILock>::Error),
}

impl<L1: ILock, L2: ILock> Nested<L1, L2>
{
    /// Creates a new `Nested` lock.
    pub fn new() -> Self
    {
        Self {
            l1: L1::default(),
            l2: L2::default(),
        }
    }
}

impl<L1: ILock, L2: ILock> core::default::Default for Nested<L1, L2>
{
    fn default() -> Self
    {
        Self::new()
    }
}

unsafe impl<L1: ILock, L2: ILock> ILock for Nested<L1, L2>
{
    type Error = Error<L1, L2>;

    fn try_lock(&self, current_iteration: usize) -> LockResult<Self::Error>
    {
        let l1 = self.l1.try_lock(current_iteration);

        if let Ok(LockStatus::Fail) = l1
        {
            return l1.map_err(|e| Error::Le1(e));
        }

        if l1.is_err()
        {
            return l1.map_err(|e| Error::Le1(e));
        }

        let l2 = self.l2.try_lock(current_iteration);

        match l2
        {
            Ok(LockStatus::Done) => Ok(LockStatus::Done),
            Ok(LockStatus::Fail) =>
            {
                self.l1.free();
                Ok(LockStatus::Fail)
            },
            Err(_) =>
            {
                self.l1.free();
                l2.map_err(|e| Error::Le2(e))
            },
        }
    }

    fn fake_lock(&self) -> LockResult<Self::Error>
    {
        let l1 = self.l1.fake_lock();

        if let Ok(LockStatus::Fail) = l1
        {
            return l1.map_err(|e| Error::Le1(e));
        }

        if l1.is_err()
        {
            return l1.map_err(|e| Error::Le1(e));
        }

        self.l2.fake_lock().map_err(|e| Error::Le2(e))
    }

    fn free(&self)
    {
        self.l2.free();
        self.l1.free();
    }

    fn wake_all(&self)
    {
        self.l1.wake_all();
        self.l2.wake_all();
    }
}
