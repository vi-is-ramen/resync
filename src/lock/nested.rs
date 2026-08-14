//! A composite lock that acquires two inner locks in a fixed order.

use crate::{ILock, LockResult};

/// A lock composed of two inner locks (`L1` and `L2`).
///
/// The `try_lock` method acquires `L1` first, and only if that succeeds,
/// it attempts to acquire `L2`. If either acquisition fails or aborts,
/// the method cleans up any already‑acquired locks.
///
/// # Deadlock Prevention
/// The order of acquisition is fixed: `L1` then `L2`. The order of release
/// (in [`ILock::free`]) is reversed: `L2` then `L1`. This helps
/// prevent deadlocks when used consistently.
#[allow(missing_debug_implementations)]
pub struct Nested<L1: ILock, L2: ILock>
{
    l1: L1,
    l2: L2,
}

impl<L1: ILock, L2: ILock> Nested<L1, L2>
{
    /// Creates a new `Nested` lock with default inner locks.
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
        Self {
            l1: L1::default(),
            l2: L2::default(),
        }
    }
}

unsafe impl<L1: ILock, L2: ILock> ILock for Nested<L1, L2>
{
    /// Attempts to acquire both inner locks in order (`L1` then `L2`).
    ///
    /// The `current_iteration` is forwarded to both inner locks.
    fn try_lock(&self, current_iteration: usize) -> LockResult
    {
        let l1 = self.l1.try_lock(current_iteration);

        if l1 == LockResult::Fail
        {
            return l1;
        }

        let l2 = self.l2.try_lock(current_iteration);

        if l1 == l2 && l2 == LockResult::Done
        {
            l1
        }
        else
        {
            if l1 == LockResult::Abort || l2 == LockResult::Abort
            {
                self.l1.free();
                self.l2.free();
                LockResult::Abort
            }
            else
            {
                self.l1.free();
                LockResult::Fail
            }
        }
    }

    fn fake_lock(&self) -> LockResult
    {
        let l1 = self.l1.fake_lock();

        if l1 == LockResult::Fail
        {
            return l1;
        }

        let l2 = self.l2.fake_lock();

        if l1 == l2 && l2 == LockResult::Done
        {
            l1
        }
        else
        {
            if l1 == LockResult::Abort || l2 == LockResult::Abort
            {
                LockResult::Abort
            }
            else
            {
                LockResult::Fail
            }
        }
    }

    /// Releases both inner locks in reverse order (`L2` then `L1`).
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

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::lock::Atomic;

    #[test]
    fn nested_acquires_both_locks_successfully()
    {
        let lock = Nested::<Atomic, Atomic>::default();
        assert_eq!(lock.try_lock(0), LockResult::Done);
        assert_eq!(lock.try_lock(0), LockResult::Fail);
        lock.free();
        assert_eq!(lock.try_lock(0), LockResult::Done);
    }
}
