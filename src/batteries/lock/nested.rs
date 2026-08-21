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
//! # use resync::api::LockPolicy;
//! # use resync::LockStatus;
//! type SafeNestedLock = Nested<Atomic, Atomic>;
//!
//! let lock = SafeNestedLock::default();
//!
//! // Acquires L1, then L2
//! assert_eq!(
//!     unsafe { lock.try_lock(0) }.unwrap(),
//!     LockStatus::Done(((), ()))
//! );
//!
//! // Releases L2, then L1
//! unsafe { lock.free(&((), ())) };
//! ```
use crate::api::{LockPolicy, NewLocked};
use crate::{LockResult, LockStatus};

/// A lock composed of two inner locks.
///
/// This struct enforces a strict acquisition and release order to prevent
/// deadlocks. The first lock (`L1`) is always acquired before the second
/// lock (`L2`), and they are released in reverse order.
#[allow(missing_debug_implementations)]
pub struct Nested<L1, L2>
where
    L1: LockPolicy,
    L2: LockPolicy,
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
where
    E1: core::error::Error,
    E2: core::error::Error,
{
    /// An error occurred in the first inner lock (`L1`).
    E1(E1),
    /// An error occurred in the second inner lock (`L2`).
    E2(E2),
}

impl<E1, E2> core::fmt::Debug for Nested<E1, E2>
where
    E1: LockPolicy + core::fmt::Debug,
    E2: LockPolicy + core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.write_fmt(format_args!("Nested {{ {:?} {:?} }}", self.l1, self.l2))
    }
}

impl<E1, E2> core::fmt::Display for NestedError<E1, E2>
where
    E1: core::error::Error + core::fmt::Display,
    E2: core::error::Error + core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        match self
        {
            Self::E1(e) => <E1 as core::fmt::Display>::fmt(e, f),
            Self::E2(e) => <E2 as core::fmt::Display>::fmt(e, f),
        }
    }
}

impl<E1, E2> core::error::Error for NestedError<E1, E2>
where
    E1: core::error::Error,
    E2: core::error::Error,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)>
    {
        match self
        {
            Self::E1(e) => e.source(),
            Self::E2(e) => e.source(),
        }
    }
}

impl<L1, L2> Nested<L1, L2>
where
    L1: LockPolicy + Default,
    L2: LockPolicy + Default,
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

impl<L1, L2> core::default::Default for Nested<L1, L2>
where
    L1: LockPolicy + Default,
    L2: LockPolicy + Default,
{
    fn default() -> Self
    {
        Self::new()
    }
}

unsafe impl<L1, L2> LockPolicy for Nested<L1, L2>
where
    L1: LockPolicy,
    L2: LockPolicy,
{
    type Error =
        NestedError<<L1 as LockPolicy>::Error, <L2 as LockPolicy>::Error>;
    type Meta = (<L1 as LockPolicy>::Meta, <L2 as LockPolicy>::Meta);

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
    ) -> LockResult<Self::Meta, Self::Error>
    {
        let l1 = unsafe { self.l1.try_lock(current_iteration) };
        let first_meta = match l1
        {
            Ok(LockStatus::Done(meta)) => meta,
            Ok(LockStatus::Fail) => return Ok(LockStatus::Fail),
            Err(error) => return Err(NestedError::E1(error)),
        };

        let l2 = unsafe { self.l2.try_lock(current_iteration) };
        match l2
        {
            Ok(LockStatus::Done(meta)) =>
            {
                Ok(LockStatus::Done((first_meta, meta)))
            },
            Ok(LockStatus::Fail) =>
            {
                unsafe { self.l1.free(&first_meta) };
                Ok(LockStatus::Fail)
            },
            Err(error) =>
            {
                unsafe { self.l1.free(&first_meta) };
                Err(NestedError::E2(error))
            },
        }
    }

    /// Releases both inner locks in reverse order (`L2` then `L1`).
    ///
    /// # Safety
    ///
    /// The caller must ensure that they currently hold both locks.
    unsafe fn free(&self, meta: &Self::Meta)
    {
        unsafe {
            self.l2.free(&meta.1);
            self.l1.free(&meta.0);
        }
    }

    /// Wakes all threads waiting on either of the inner locks.
    fn wake_all(&self)
    {
        self.l1.wake_all();
        self.l2.wake_all();
    }
}

impl<L1, L2> NewLocked for Nested<L1, L2>
where
    L1: NewLocked,
    L2: NewLocked,
{
    /// Creates a new [`Nested`] lock with both inner locks already acquired.
    ///
    /// `L1` is acquired first, followed by `L2`, maintaining the strict
    /// deterministic order enforced by this primitive.
    fn new_locked() -> (Self::Meta, Self)
    {
        let (m1, l1) = L1::new_locked();
        let (m2, l2) = L2::new_locked();
        ((m1, m2), Self { l1, l2 })
    }
}
