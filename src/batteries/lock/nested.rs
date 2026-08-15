//! A composite lock that acquires two inner locks in a fixed, deterministic
//! order.
//!
//! This module provides the [`Nested`] lock, which is designed to help prevent
//! deadlocks when multiple locks need to be acquired simultaneously. Deadlocks
//! often occur when different threads acquire the same set of locks in
//! different orders. By wrapping two locks in a `Nested` struct, the
//! acquisition order is strictly enforced at compile time: `L1` is always
//! acquired before `L2`, and `L2` is always released before `L1`.
//!
//! # Examples
//!
//! ```rust
//! # use resync::lock::{Atomic, Nested};
//! # use resync::traits::LockPolicy;
//! # use resync::LockStatus;
//! type SafeNestedLock = Nested<Atomic, Atomic>;
//!
//! let lock = SafeNestedLock::default();
//!
//! // Acquires L1, then L2
//! assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done);
//!
//! // Releases L2, then L1
//! unsafe { lock.free() };
//! ```

use crate::traits::LockPolicy;
use crate::{LockResult, LockStatus};

/// A lock composed of two inner locks.
///
/// This struct enforces a strict acquisition and release order to prevent
/// deadlocks. The first lock (`L1`) is always acquired before the second
/// lock (`L2`), and they are released in reverse order.
#[allow(missing_debug_implementations)]
pub struct Nested<L1: LockPolicy, L2: LockPolicy>
{
    l1: L1,
    l2: L2,
}

/// Errors that can occur when acquiring or inspecting a [`Nested`] lock.
///
/// Because a nested lock consists of two inner locks, an error can originate
/// from either the first lock (`Le1`) or the second lock (`Le2`).
#[derive(Debug)]
pub enum NestedError<E1, E2>
{
    /// An error occurred in the first inner lock (`L1`).
    E1(E1),

    /// An error occurred in the second inner lock (`L2`).
    E2(E2),
}

impl<L1: LockPolicy, L2: LockPolicy> Nested<L1, L2>
{
    /// Creates a new [`Nested`] lock with default-constructed inner locks.
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
    type Error =
        NestedError<<L1 as LockPolicy>::Error, <L2 as LockPolicy>::Error>;

    /// Attempts to acquire both inner locks in order (`L1` then `L2`).
    ///
    /// If `L1` fails to acquire, it returns immediately. If `L1` succeeds but
    /// `L2` fails, it automatically releases `L1` before returning to ensure
    /// no locks are left dangling.
    ///
    /// # Safety
    ///
    /// The caller must ensure proper memory ordering when accessing protected
    /// data.
    unsafe fn try_lock(
        &self,
        current_iteration: usize,
    ) -> LockResult<Self::Error>
    {
        let l1 = unsafe { self.l1.try_lock(current_iteration) };

        if let Ok(LockStatus::Fail) = l1
        {
            return l1.map_err(NestedError::E1);
        }

        if l1.is_err()
        {
            return l1.map_err(NestedError::E1);
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
                l2.map_err(NestedError::E2)
            },
        }
    }

    /// Checks the state of both inner locks without modifying them.
    ///
    /// Returns [`LockStatus::Fail`] if either lock is currently held.
    fn get_state(&self) -> LockResult<Self::Error>
    {
        let l1 = self.l1.get_state();

        if let Ok(LockStatus::Fail) = l1
        {
            return l1.map_err(NestedError::E1);
        }

        if l1.is_err()
        {
            return l1.map_err(NestedError::E1);
        }

        self.l2.get_state().map_err(NestedError::E2)
    }

    /// Releases both inner locks in reverse order (`L2` then `L1`).
    ///
    /// # Safety
    ///
    /// The caller must ensure that they currently hold both locks.
    unsafe fn free(&self)
    {
        unsafe {
            self.l2.free();
            self.l1.free();
        }
    }

    /// Wakes all threads waiting on either of the inner locks.
    fn wake_all(&self)
    {
        self.l1.wake_all();
        self.l2.wake_all();
    }
}
