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
///
/// # Type Parameters
/// - `L1`: the first (outer) lock type.
/// - `L2`: the second (inner) lock type.
#[allow(missing_debug_implementations)]
pub struct Nested<L1: ILock, L2: ILock>
{
    l1: L1,
    l2: L2,
}

impl<L1: ILock, L2: ILock> Nested<L1, L2>
{
    /// Creates a new `Nested` lock with default inner locks.
    ///
    /// # Panics
    /// This method does not panic, but relies on `L1::default()` and
    /// `L2::default()` not to panic.
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
    /// # Returns
    /// - [`LockResult::Done`]  – both locks were acquired successfully.
    /// - `Fail`  – `L1` was already held, or `L1` was acquired but `L2` was
    ///   already held.
    /// - `Abort` – either `L1` or `L2` returned `Abort`; any acquired locks are
    ///   released before returning.
    ///
    /// # Errors
    /// If an abort occurs, the state is cleaned up and `Abort` is returned.
    fn try_lock(&self) -> LockResult
    {
        // lock L1
        let l1 = self.l1.try_lock();

        // early return if L1 is Fail
        if l1 == LockResult::Fail
        {
            return l1;
        }

        // lock l2
        let l2 = self.l2.try_lock();

        // check if both are Done
        if l1 == l2 && l2 == LockResult::Done
        {
            l1
        }
        else
        {
            // if Abort in one of the results, reset both locks and return Abort
            if l1 == LockResult::Abort || l2 == LockResult::Abort
            {
                self.l1.free();
                self.l2.free();
                LockResult::Abort
            }
            else
            {
                // here, L1 returned Done and L2 returned Fail.
                // we need to reset L1 (otherwise deadlock possible)
                // and return Fail.
                self.l1.free();
                LockResult::Fail
            }
        }
    }

    fn fake_lock(&self) -> LockResult
    {
        // NOTE:
        // same logic as in `try_lock` but without `free`s as we don't change
        // states of locks.

        // lock L1
        let l1 = self.l1.fake_lock();

        // early return if L1 is Fail
        if l1 == LockResult::Fail
        {
            return l1;
        }

        // lock l2
        let l2 = self.l2.fake_lock();

        // check if both are Done
        if l1 == l2 && l2 == LockResult::Done
        {
            l1
        }
        else
        {
            // if Abort in one of the results, reset both locks and return Abort
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
    ///
    /// This ordering helps prevent deadlocks when paired with the acquisition
    /// order. The method is idempotent.
    fn free(&self)
    {
        // reverse order - deadlock possible otherwise
        self.l2.free();
        self.l1.free();
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
        assert_eq!(lock.try_lock(), LockResult::Done);
        // Both inner locks are now held; trying again fails.
        assert_eq!(lock.try_lock(), LockResult::Fail);
        lock.free();
        assert_eq!(lock.try_lock(), LockResult::Done);
    }
}
