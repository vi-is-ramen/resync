use crate::api::PoisonPolicy;

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
    #[inline]
    fn is_poisoned(&self) -> bool
    {
        false
    }

    #[inline]
    fn on_drop(&self) {}

    #[inline]
    unsafe fn clear_poison(&self) {}
}
