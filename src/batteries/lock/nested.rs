//! A composite lock that acquires two inner locks in a fixed order.

use crate::traits::LockPolicy;
use crate::{LockResult, LockStatus};

/// A lock composed of two inner locks.
#[allow(missing_debug_implementations)]
pub struct Nested<L1: LockPolicy, L2: LockPolicy>
{
    l1: L1,
    l2: L2,
}

/// .
#[allow(missing_debug_implementations)]
pub enum NestedError<L2: LockPolicy, L1: LockPolicy>
{
    /// .
    Le1(<L1 as LockPolicy>::Error),

    /// .
    Le2(<L2 as LockPolicy>::Error),
}

impl<L1: LockPolicy, L2: LockPolicy> Nested<L1, L2>
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

impl<L1: LockPolicy, L2: LockPolicy> core::default::Default for Nested<L1, L2>
{
    fn default() -> Self
    {
        Self::new()
    }
}

unsafe impl<L1: LockPolicy, L2: LockPolicy> LockPolicy for Nested<L1, L2>
{
    type Error = NestedError<L2, L1>;

    unsafe fn try_lock(
        &self,
        current_iteration: usize,
    ) -> LockResult<Self::Error>
    {
        let l1 = unsafe { self.l1.try_lock(current_iteration) };

        if let Ok(LockStatus::Fail) = l1
        {
            return l1.map_err(|e| NestedError::Le1(e));
        }

        if l1.is_err()
        {
            return l1.map_err(|e| NestedError::Le1(e));
        }

        let l2 = unsafe { self.l2.try_lock(current_iteration) };

        match l2
        {
            Ok(LockStatus::Done) => Ok(LockStatus::Done),
            Ok(LockStatus::Fail) =>
            {
                unsafe { self.l1.free() };
                Ok(LockStatus::Fail)
            },
            Err(_) =>
            {
                unsafe { self.l1.free() };
                l2.map_err(|e| NestedError::Le2(e))
            },
        }
    }

    fn get_state(&self) -> LockResult<Self::Error>
    {
        let l1 = self.l1.get_state();

        if let Ok(LockStatus::Fail) = l1
        {
            return l1.map_err(|e| NestedError::Le1(e));
        }

        if l1.is_err()
        {
            return l1.map_err(|e| NestedError::Le1(e));
        }

        self.l2.get_state().map_err(|e| NestedError::Le2(e))
    }

    unsafe fn free(&self)
    {
        unsafe {
            self.l2.free();
            self.l1.free();
        }
    }

    fn wake_all(&self)
    {
        self.l1.wake_all();
        self.l2.wake_all();
    }
}
