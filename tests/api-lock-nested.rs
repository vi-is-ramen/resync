//! Characterization: lock::Nested composite lock and NestedError.

use core::convert::Infallible;
use resync::LockStatus;
use resync::api::{LockPolicy, NewLocked};
use resync::lock::{Atomic, Nested, NestedError};

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

type NestedAtomic = Nested<Atomic, Atomic>;

// Compile-time: Meta is a tuple of inner metas; Error is NestedError.
#[allow(dead_code)]
fn _assert_associated_types()
{
    fn check<
        L: LockPolicy<
                Meta = ((), ()),
                Error = NestedError<Infallible, Infallible>,
            >,
    >()
    {
    }
    check::<NestedAtomic>();
}

#[test]
fn nested_acquires_and_releases_both()
{
    let lock = NestedAtomic::new();
    assert_eq!(
        unsafe { lock.try_lock(0) }.unwrap(),
        LockStatus::Done(((), ()))
    );
    // Second acquisition fails (inner locks held).
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Fail);
    unsafe { lock.free(&((), ())) };
    assert_eq!(
        unsafe { lock.try_lock(0) }.unwrap(),
        LockStatus::Done(((), ()))
    );
    unsafe { lock.free(&((), ())) };
}

#[test]
fn nested_new_locked()
{
    let (meta, lock) = NestedAtomic::new_locked();
    assert_eq!(meta, ((), ()));
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Fail);
    unsafe { lock.free(&meta) };
    assert_eq!(
        unsafe { lock.try_lock(0) }.unwrap(),
        LockStatus::Done(((), ()))
    );
    unsafe { lock.free(&((), ())) };
}

#[test]
fn nested_default_and_debug()
{
    let lock = NestedAtomic::default();
    let _ = format!("{lock:?}");
    assert_eq!(
        unsafe { lock.try_lock(0) }.unwrap(),
        LockStatus::Done(((), ()))
    );
    unsafe { lock.free(&((), ())) };
}

#[test]
fn nested_error_variants_display_source()
{
    use core::error::Error;
    let e1: NestedError<TestErr, TestErr> = NestedError::E1(TestErr);
    let e2: NestedError<TestErr, TestErr> = NestedError::E2(TestErr);
    assert!(matches!(e1, NestedError::E1(_)));
    assert!(matches!(e2, NestedError::E2(_)));
    assert_eq!(format!("{e1}"), "test error");
    assert_eq!(format!("{e2}"), "test error");
    assert!(e1.source().is_none());
    assert!(e2.source().is_none());
}
