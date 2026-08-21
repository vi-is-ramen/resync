//! Characterization: lock::Shield writer-fairness wrapper and ShieldError.

use resync::LockStatus;
use resync::api::{LockPolicy, NewLocked, SharingPolicy};
use resync::lock::{Atomic, Shield, ShieldError};

#[derive(Debug)]
struct TestErr;

impl core::fmt::Display for TestErr
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.write_str("test error")
    }
}

impl core::error::Error for TestErr {}

type ShieldAtomic = Shield<Atomic>;

// Compile-time: Shield<L> forwards Meta and wraps Error into ShieldError.
#[allow(dead_code)]
fn _assert_associated_types()
{
    fn check<
        L: LockPolicy<Meta = (), Error = ShieldError<core::convert::Infallible>>,
    >()
    {
    }
    check::<ShieldAtomic>();
    fn check_new_locked<L: NewLocked>() {}
    check_new_locked::<ShieldAtomic>();
}

#[test]
fn shield_basic_exclusive_cycle()
{
    let s = ShieldAtomic::default();
    assert_eq!(unsafe { s.try_lock(0) }.unwrap(), LockStatus::Done(()));
    assert_eq!(unsafe { s.try_lock(0) }.unwrap(), LockStatus::Fail);
    unsafe { s.free(&()) };
}

#[test]
fn shield_reader_then_writer_contention()
{
    let s = ShieldAtomic::default();
    // Reader holds the lock.
    assert_eq!(s.try_share(0).unwrap(), LockStatus::Done(()));
    // Writer fails and becomes "pending".
    assert_eq!(unsafe { s.try_lock(1) }.unwrap(), LockStatus::Fail);
    // New readers are now blocked even though only a reader holds the lock.
    assert_eq!(s.try_share(0).unwrap(), LockStatus::Fail);
    // Release the reader -> writer can proceed.
    s.free_share(&());
    assert_eq!(unsafe { s.try_lock(1) }.unwrap(), LockStatus::Done(()));
    unsafe { s.free(&()) };
    // Pending counter drained -> readers pass again.
    assert_eq!(s.try_share(0).unwrap(), LockStatus::Done(()));
    s.free_share(&());
}

#[test]
fn shield_new_locked()
{
    let (meta, s) = ShieldAtomic::new_locked();
    assert_eq!(unsafe { s.try_lock(1) }.unwrap(), LockStatus::Fail);
    unsafe { s.free(&meta) };
}

#[test]
fn shield_error_variants()
{
    let writer: ShieldError<TestErr> = ShieldError::Writer;
    let lock: ShieldError<TestErr> = ShieldError::Lock(TestErr);
    assert!(matches!(writer, ShieldError::Writer));
    assert!(matches!(lock, ShieldError::Lock(_)));
    assert_eq!(format!("{writer}"), "Writer is waiting for this resource");
    assert_eq!(format!("{lock}"), "test error");
}
