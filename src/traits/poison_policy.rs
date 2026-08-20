//! Policies for handling lock poisoning.
//!
//! This module provides the [`PoisonPolicy`] trait, which abstracts the
//! mechanism for detecting thread panics and marking a lock as poisoned.
//!
//! # Purpose
//!
//! In standard Rust, locks become "poisoned" if a thread panics while holding
//! them, signaling to subsequent threads that the protected data might be in
//! an inconsistent state. However, this mechanism relies on
//! `std::thread::panicking()`, which is unavailable in `#![no_std]`
//! environments.
//!
//! By abstracting poisoning into a policy, `resync` allows:
//! - **Zero-overhead locks**: Using [`crate::poison::NoPoison`] completely
//!   eliminates the atomic flag and panic-checking overhead for locks where
//!   poisoning is unnecessary.
//! - **Custom panic detection**: Users in `#![no_std]` environments with custom
//!   unwinding or panic-tracking mechanisms can implement their own
//!   [`PoisonPolicy`] to enable poisoning without depending on `std`.

/// A policy that defines how a lock handles thread panics (poisoning).
///
/// Implementations of this trait determine whether a lock should be marked
/// as poisoned when a thread panics while holding it, and how to store and
/// query this poisoned state.
// NOTE: This trait **must** be dyn-compatible by design.
pub trait PoisonPolicy
{
    /// Returns `true` if the lock is currently poisoned.
    fn is_poisoned(&self) -> bool;

    /// Called when a guard is dropped.
    ///
    /// Implementations should check if the current thread is panicking
    /// (or use their own custom panic detection mechanism) and update the
    /// `state` accordingly.
    fn on_drop(&self);

    /// Clears the poisoned state.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the protected data has been manually
    /// repaired or validated before calling this method.
    unsafe fn clear_poison(&self);
}
