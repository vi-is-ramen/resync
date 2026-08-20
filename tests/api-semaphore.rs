//! Characterization: Semaphore counting permits.
#![cfg(feature = "dev")]

use resync::lock::Atomic;
use resync::retry::Busy;
use resync::{Semaphore, TryLockError};

type TestSem = Semaphore<Atomic, Busy>;

#[test]
fn semaphore_new_and_permits()
{
    let s = TestSem::new(3);
    assert_eq!(s.available_permits().unwrap(), 3);
}

#[test]
fn semaphore_acquire_release_cycle()
{
    let s = TestSem::new(2);
    s.acquire().unwrap();
    assert_eq!(s.available_permits().unwrap(), 1);
    s.acquire().unwrap();
    assert_eq!(s.available_permits().unwrap(), 0);
    // No permits left.
    assert!(matches!(s.try_acquire(), Err(TryLockError::Contention)));
    s.release().unwrap();
    assert_eq!(s.available_permits().unwrap(), 1);
    s.release().unwrap();
    assert_eq!(s.available_permits().unwrap(), 2);
}

#[test]
fn semaphore_acquire_many_and_release_many()
{
    let s = TestSem::new(5);
    s.acquire_many(3).unwrap();
    assert_eq!(s.available_permits().unwrap(), 2);
    // Not enough permits for 3 more.
    assert!(matches!(
        s.try_acquire_many(3),
        Err(TryLockError::Contention)
    ));
    s.release_many(3).unwrap();
    assert_eq!(s.available_permits().unwrap(), 5);
}

#[test]
fn semaphore_default_has_zero_permits()
{
    let s = TestSem::default();
    assert_eq!(s.available_permits().unwrap(), 0);
    assert!(matches!(s.try_acquire(), Err(TryLockError::Contention)));
}

#[test]
fn semaphore_from_tuple()
{
    let s = Semaphore::from((4, Atomic::default(), Busy));
    assert_eq!(s.available_permits().unwrap(), 4);
}
