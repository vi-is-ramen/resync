//! Built-in implementations of [`PoisonPolicy`](crate::traits::PoisonPolicy).

use crate::traits::PoisonPolicy;
use core::sync::atomic::{AtomicBool, Ordering};

/// A poison policy that never poisons the lock.
///
/// This policy has zero overhead and is ideal for `#![no_std]` environments
/// without panic unwinding, or for locks where the user can guarantee that
/// the protected data will never be left in an inconsistent state during a
/// panic.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoPoison;

impl PoisonPolicy for NoPoison
{
    type State = ();

    #[inline]
    fn new_state() -> Self::State {}

    #[inline]
    fn is_poisoned(_: &Self::State) -> bool
    {
        false
    }

    #[inline]
    fn on_drop(_: &Self::State) {}

    #[inline]
    unsafe fn clear_poison(_: &Self::State) {}
}

/// A poison policy that uses `std::thread::panicking()` to detect panics.
///
/// This is the default policy when the `std` feature is enabled. It stores
/// the poisoned state in an [`AtomicBool`].
#[cfg(feature = "std")]
#[derive(Debug, Default, Clone, Copy)]
pub struct StdPoison;

#[cfg(feature = "std")]
impl PoisonPolicy for StdPoison
{
    type State = AtomicBool;

    #[inline]
    fn new_state() -> Self::State
    {
        AtomicBool::new(false)
    }

    #[inline]
    fn is_poisoned(state: &Self::State) -> bool
    {
        state.load(Ordering::Acquire)
    }

    #[inline]
    fn on_drop(state: &Self::State)
    {
        if std::thread::panicking()
        {
            state.store(true, Ordering::Release);
        }
    }

    #[inline]
    unsafe fn clear_poison(state: &Self::State)
    {
        state.store(false, Ordering::Release);
    }
}

/// The default poison policy.
///
/// - When the `std` feature is enabled, this is an alias for [`StdPoison`].
/// - When the `std` feature is disabled, this is an alias for [`NoPoison`].
#[cfg(feature = "std")]
pub type DefaultPoison = StdPoison;

#[cfg(not(feature = "std"))]
pub type DefaultPoison = NoPoison;
