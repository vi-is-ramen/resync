//! A fake lock and retry policy implementation for testing purposes.
//!
//! This module provides [`Fake`], a mock implementation of [`LockPolicy`],
//! [`SharingPolicy`], [`RetryPolicy`], and [`PoisonPolicy`]. It always succeeds
//! in acquiring the lock and never blocks or yields, making it useful for unit
//! testing higher-level primitives without dealing with actual concurrency or
//! contention.

use crate::RetryResult;
use crate::traits::{
    LockPolicy, NewLocked, PoisonPolicy, RetryPolicy, SharingPolicy,
};

/// A fake lock and retry policy that always succeeds.
///
/// This is primarily intended for testing scenarios where you need a lock
/// policy that guarantees immediate, uncontended acquisition without any
/// actual synchronization overhead.
#[derive(Debug, Default)]
pub struct Fake;

/// An error type for the [`Fake`] policy.
///
/// Since [`Fake`] never actually fails, this error type is theoretically
/// unreachable in normal operation. It exists solely to satisfy the
/// associated `Display` type requirements of the policy traits.
// NOTE: Infallible?
#[derive(Debug, Default)]
pub struct FakeError;

impl core::fmt::Display for FakeError
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.write_str("FakeErr")
    }
}

unsafe impl core::marker::Sync for Fake {}
impl core::error::Error for FakeError {}

unsafe impl LockPolicy for Fake
{
    type Error = FakeError;
    type Meta = ();

    /// Always succeeds in acquiring the exclusive lock.
    unsafe fn try_lock(
        &self,
        _current_iteration: usize,
    ) -> crate::LockResult<Self::Meta, Self::Error>
    {
        Ok(crate::LockStatus::Done(()))
    }

    /// No-op release operation.
    unsafe fn free(&self, _: &Self::Meta) {}

    /// No-op wake operation.
    fn wake_all(&self) {}
}

impl NewLocked for Fake
{
    /// Creates a new `Fake` lock.
    ///
    /// Since `Fake` always succeeds on `try_lock`, this is functionally
    /// identical to `Fake::default()`. It exists to satisfy the `NewLocked`
    /// trait bound in testing scenarios.
    fn new_locked() -> (Self::Meta, Self)
    {
        ((), Self)
    }
}

unsafe impl SharingPolicy for Fake
{
    /// Always succeeds in acquiring the shared lock.
    fn try_share(
        &self,
        _current_iteration: usize,
    ) -> crate::LockResult<Self::Meta, Self::Error>
    {
        Ok(crate::LockStatus::Done(()))
    }

    /// No-op release operation.
    fn free_share(&self, _: &Self::Meta) {}

    /// No-op wake operation.
    fn wake_readers(&self) {}
}

impl RetryPolicy for Fake
{
    type Error = FakeError;

    /// Always succeeds and continues the retry loop (though it will never
    /// actually be called since `try_lock` and `try_share` always succeed).
    fn retry(
        &self,
        _current_iteration: usize,
    ) -> crate::RetryResult<Self::Error>
    {
        RetryResult::Ok(())
    }
}

impl PoisonPolicy for Fake
{
    unsafe fn clear_poison(&self) {}

    fn is_poisoned(&self) -> bool
    {
        false
    }

    fn on_drop(&self) {}
}
