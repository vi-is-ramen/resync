//! Characterization: lock::Atomic backend (LockPolicy + SharingPolicy +
//! NewLocked).

use core::convert::Infallible;
use resync::LockStatus;
use resync::lock::Atomic;
use resync::traits::{LockPolicy, NewLocked, SharingPolicy};

// Compile-time: associated types are pinned to Infallible / ().
#[allow(dead_code)]
fn _assert_associated_types()
{
    fn check_lock<L: LockPolicy<Error = Infallible, Meta = ()>>() {}
    fn check_share<L: SharingPolicy<Error = Infallible, Meta = ()>>() {}
    check_lock::<Atomic>();
    check_share::<Atomic>();
}

#[test]
fn atomic_new_is_free_and_exclusive_cycle()
{
    let lock = Atomic::new();
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    // Re-entrant acquisition must fail.
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Fail);
    unsafe { lock.free(&()) };
    // After release it is acquirable again.
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    unsafe { lock.free(&()) };
}

#[test]
fn atomic_writer_blocks_readers_and_writers()
{
    let lock = Atomic::new();
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    assert_eq!(lock.try_share(0).unwrap(), LockStatus::Fail);
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Fail);
    unsafe { lock.free(&()) };
}

#[test]
fn atomic_multiple_readers_block_writer()
{
    let lock = Atomic::new();
    assert_eq!(lock.try_share(0).unwrap(), LockStatus::Done(()));
    assert_eq!(lock.try_share(0).unwrap(), LockStatus::Done(()));
    // Writer blocked while at least one reader holds the lock.
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Fail);
    lock.free_share(&());
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Fail);
    lock.free_share(&());
    // Now free for a writer.
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    unsafe { lock.free(&()) };
}

#[test]
fn atomic_new_locked_starts_acquired()
{
    let (meta, lock) = Atomic::new_locked();
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Fail);
    assert_eq!(lock.try_share(0).unwrap(), LockStatus::Fail);
    unsafe { lock.free(&meta) };
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    unsafe { lock.free(&()) };
}

#[test]
fn atomic_default_and_debug()
{
    let lock = Atomic::default();
    let _ = format!("{lock:?}");
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    unsafe { lock.free(&()) };
}
