//! Characterization: core result/error taxonomy.
//! LockStatus, LockResult, RetryResult, PoisonError, AcquireError,
//! TryLockError.

use core::convert::Infallible;
use core::error::Error;
use resync::{
    AcquireError, LockResult, LockStatus, PoisonError, RetryResult,
    TryLockError,
};

#[derive(Debug)]
struct TestErr;

impl core::fmt::Display for TestErr
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.write_str("test error")
    }
}

impl Error for TestErr {}

// --- LockStatus ---

#[test]
fn lock_status_variants_and_eq()
{
    let fail: LockStatus<()> = LockStatus::Fail;
    let done: LockStatus<u32> = LockStatus::Done(42);
    assert_eq!(fail, LockStatus::Fail);
    assert_eq!(done, LockStatus::Done(42));
    assert_ne!(done, LockStatus::Done(43));
}

#[test]
fn lock_status_copy_clone()
{
    let a: LockStatus<u32> = LockStatus::Done(7);
    let b = a; // Copy
    #[allow(clippy::clone_on_copy)]
    let c = a.clone(); // Clone
    assert_eq!(a, b);
    assert_eq!(a, c);
}

#[test]
fn lock_status_ordering()
{
    // `Fail` is declared before `Done`, so it must compare less.
    let fail: LockStatus<u32> = LockStatus::Fail;
    let done: LockStatus<u32> = LockStatus::Done(0);
    assert!(fail < done);
}

#[test]
fn lock_status_hash_and_debug()
{
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(LockStatus::<u32>::Fail);
    set.insert(LockStatus::<u32>::Done(1));
    set.insert(LockStatus::<u32>::Done(1));
    assert_eq!(set.len(), 2);
    let _ = format!("{:?}", LockStatus::<()>::Fail);
}

// --- LockResult / RetryResult aliases ---

#[test]
fn result_aliases()
{
    let lr: LockResult<u32, TestErr> = Ok(LockStatus::Done(1));
    #[allow(clippy::unnecessary_literal_unwrap)]
    {
        assert_eq!(lr.unwrap(), LockStatus::Done(1));
    }

    let lr_err: LockResult<u32, TestErr> = Err(TestErr);
    assert!(lr_err.is_err());

    let rr: RetryResult<TestErr> = Ok(());
    assert!(rr.is_ok());

    let rr_err: RetryResult<TestErr> = Err(TestErr);
    assert!(rr_err.is_err());
}

// --- PoisonError ---

#[test]
fn poison_error_new_and_into_inner()
{
    let pe = PoisonError::new(42u32);
    assert_eq!(pe.into_inner(), 42);
}

#[test]
fn poison_error_display_and_source()
{
    let pe = PoisonError::new(());
    assert_eq!(format!("{pe}"), "lock poisoned");
    let e: &dyn Error = &pe;
    assert!(e.source().is_none());
}

// --- AcquireError ---

#[test]
fn acquire_error_variants()
{
    let poisoned: AcquireError<u32, TestErr, TestErr> =
        AcquireError::Poisoned(PoisonError::new(1));
    let lock: AcquireError<u32, TestErr, TestErr> = AcquireError::Lock(TestErr);
    let retry: AcquireError<u32, TestErr, TestErr> =
        AcquireError::Retry(TestErr);
    assert!(matches!(poisoned, AcquireError::Poisoned(_)));
    assert!(matches!(lock, AcquireError::Lock(_)));
    assert!(matches!(retry, AcquireError::Retry(_)));
}

#[test]
fn acquire_error_display()
{
    let poisoned: AcquireError<u32, TestErr, TestErr> =
        AcquireError::Poisoned(PoisonError::new(1));
    let lock: AcquireError<u32, TestErr, TestErr> = AcquireError::Lock(TestErr);
    let retry: AcquireError<u32, TestErr, TestErr> =
        AcquireError::Retry(TestErr);
    assert_eq!(format!("{poisoned}"), "lock poisoned");
    assert_eq!(format!("{lock}"), "test error");
    assert_eq!(format!("{retry}"), "test error");
}

#[test]
fn acquire_error_source()
{
    let poisoned: AcquireError<u32, TestErr, TestErr> =
        AcquireError::Poisoned(PoisonError::new(1));
    let lock: AcquireError<u32, TestErr, TestErr> = AcquireError::Lock(TestErr);
    let retry: AcquireError<u32, TestErr, TestErr> =
        AcquireError::Retry(TestErr);
    // Poisoned exposes the inner PoisonError; Lock/Retry delegate to their
    // error.
    assert!(poisoned.source().is_some());
    assert!(lock.source().is_none());
    assert!(retry.source().is_none());
}

#[test]
fn acquire_error_over_infallible_only_poisoned()
{
    // Compile-time proof: with Infallible lock/retry errors only Poisoned is
    // inhabitable.
    fn only_poisoned(
        e: AcquireError<(), Infallible, Infallible>,
    ) -> PoisonError<()>
    {
        match e
        {
            AcquireError::Poisoned(pe) => pe,
            AcquireError::Lock(x) => match x {},
            AcquireError::Retry(x) => match x {},
        }
    }
    let e: AcquireError<(), Infallible, Infallible> =
        AcquireError::Poisoned(PoisonError::new(()));
    assert_eq!(only_poisoned(e).into_inner(), ());
}

// --- TryLockError ---

#[test]
fn try_lock_error_variants()
{
    let contention: TryLockError<u32, TestErr> = TryLockError::Contention;
    let lock: TryLockError<u32, TestErr> = TryLockError::Lock(TestErr);
    let poisoned: TryLockError<u32, TestErr> =
        TryLockError::Poisoned(PoisonError::new(1));
    assert!(matches!(contention, TryLockError::Contention));
    assert!(matches!(lock, TryLockError::Lock(_)));
    assert!(matches!(poisoned, TryLockError::Poisoned(_)));
}

#[test]
fn try_lock_error_display()
{
    let contention: TryLockError<u32, TestErr> = TryLockError::Contention;
    let lock: TryLockError<u32, TestErr> = TryLockError::Lock(TestErr);
    let poisoned: TryLockError<u32, TestErr> =
        TryLockError::Poisoned(PoisonError::new(1));
    assert_eq!(format!("{contention}"), "lock contention");
    assert_eq!(format!("{lock}"), "test error");
    assert_eq!(format!("{poisoned}"), "lock poisoned");
}

#[test]
fn try_lock_error_source()
{
    let contention: TryLockError<u32, TestErr> = TryLockError::Contention;
    let lock: TryLockError<u32, TestErr> = TryLockError::Lock(TestErr);
    let poisoned: TryLockError<u32, TestErr> =
        TryLockError::Poisoned(PoisonError::new(1));
    assert!(contention.source().is_none());
    assert!(lock.source().is_none());
    assert!(poisoned.source().is_some());
}
