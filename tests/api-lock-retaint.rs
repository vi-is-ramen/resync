//! Characterization: lock::Retaint reentrant (recursive) lock wrapper.
//!
//! This test pins down the public API surface of [`Retaint`]:
//! - It forwards `Error` and `Meta` from the wrapped lock policy.
//! - It implements both `LockPolicy` and `SharingPolicy`.
//! - Exclusive acquisition is reentrant for the owning thread.
//! - Downgrading from exclusive to shared is rejected.
//! - A custom `StableThreadId` provider is accepted.
#![cfg(all(feature = "dev", feature = "std"))]

use core::convert::Infallible;
use resync::LockStatus;
use resync::api::{LockPolicy, SharingPolicy, StableThreadId};
use resync::lock::{Atomic, Retaint};

type RetaintAtomic = Retaint<Atomic>;

// Compile-time: Retaint<L> forwards `Error` and `Meta` from the inner lock
// and implements both exclusive and sharing policies.
#[allow(dead_code)]
fn _assert_associated_types()
{
    fn check_lock<L: LockPolicy<Error = Infallible, Meta = ()>>() {}
    fn check_share<L: SharingPolicy>() {}
    check_lock::<RetaintAtomic>();
    check_share::<RetaintAtomic>();
}

// Compile-time: the default thread-id provider yields a usable policy.
#[allow(dead_code)]
fn _assert_default_provider_is_usable()
{
    fn check_lock<L: LockPolicy + Sync>() {}
    fn check_share<L: SharingPolicy>() {}
    check_lock::<Retaint<Atomic>>();
    check_share::<Retaint<Atomic>>();
}

// A custom thread-id provider. Characterizes the `StableThreadId` trait
// surface: `type Id: Eq` and `fn thread_id() -> Self::Id`.
struct SingleThread;

unsafe impl StableThreadId for SingleThread
{
    type Id = usize;

    fn thread_id() -> usize
    {
        7
    }
}

// Compile-time: Retaint accepts a custom thread-id provider.
#[allow(dead_code)]
fn _assert_custom_provider_is_usable()
{
    fn check_lock<L: LockPolicy>() {}
    check_lock::<Retaint<Atomic, SingleThread>>();
}

#[test]
fn retaint_reentrant_exclusive_cycle()
{
    let lock = RetaintAtomic::default();

    // First acquisition delegates to the inner lock.
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    // Reentrant acquisitions by the same thread succeed immediately.
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));

    // Release all three levels; only the last one frees the inner lock.
    unsafe { lock.free(&()) };
    unsafe { lock.free(&()) };
    unsafe { lock.free(&()) };

    // Fully released: a fresh acquisition succeeds again.
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    unsafe { lock.free(&()) };
}

#[test]
fn retaint_partial_release_keeps_lock_held()
{
    let lock = RetaintAtomic::default();

    // Acquire twice.
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));

    // Release only one level; the lock is still held (reentrancy works).
    unsafe { lock.free(&()) };
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));

    // Clean up the two remaining levels.
    unsafe { lock.free(&()) };
    unsafe { lock.free(&()) };
}

#[test]
fn retaint_exclusive_owner_cannot_downgrade_to_shared()
{
    let lock = RetaintAtomic::default();

    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    // Downgrading to shared access is not supported.
    assert_eq!(lock.try_share(0).unwrap(), LockStatus::Fail);
    unsafe { lock.free(&()) };

    // After a full release, a shared acquisition delegates to the inner
    // lock and succeeds.
    assert_eq!(lock.try_share(0).unwrap(), LockStatus::Done(()));
    lock.free_share(&());
}

#[test]
fn retaint_shared_delegates_when_not_exclusively_owned()
{
    let lock = RetaintAtomic::default();

    // Multiple shared acquisitions are forwarded to the inner lock.
    assert_eq!(lock.try_share(0).unwrap(), LockStatus::Done(()));
    assert_eq!(lock.try_share(0).unwrap(), LockStatus::Done(()));
    lock.free_share(&());
    lock.free_share(&());
}

#[test]
fn retaint_works_with_custom_thread_id_provider()
{
    let lock = Retaint::<Atomic, SingleThread>::default();

    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    // Same "thread" (constant id) can reenter.
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    unsafe { lock.free(&()) };
    unsafe { lock.free(&()) };
}

#[test]
fn retaint_default_and_debug()
{
    let lock = RetaintAtomic::default();
    let _ = format!("{lock:?}");
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    unsafe { lock.free(&()) };
}
