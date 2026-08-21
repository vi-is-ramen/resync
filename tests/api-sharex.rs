//! Characterization: Sharex (read-write) primitive + ShGuard/ExGuard.

use resync::api::{PoisonPolicy, RetryPolicy, SharingPolicy};
use resync::lock::Atomic;
use resync::poison::NoPoison;
use resync::retry::Busy;
use resync::{ExGuard, ShGuard, Sharex, TryLockError};

type TestSharex = Sharex<u32, Atomic, Busy, NoPoison>;

// --- Compile-time: default type parameters ---
#[allow(dead_code)]
fn _assert_default_type_params()
{
    let _: Sharex<u32> = Sharex::new(0);
    let _: Sharex<u32, Atomic> = Sharex::new(0);
    let _: Sharex<u32, Atomic, Busy> = Sharex::new(0);
    let _: Sharex<u32, Atomic, Busy, NoPoison> = Sharex::new(0);
}

#[allow(dead_code)]
fn _assert_generic_bounds<T, L, R, P>()
where
    T: Default,
    L: SharingPolicy + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy + Default,
{
    let _: Sharex<T, L, R, P> = Sharex::new(T::default());
}

#[allow(dead_code)]
fn _assert_guard_types(s: &TestSharex)
{
    let _: Result<ShGuard<'_, u32, Atomic, NoPoison>, _> = s.read();
    let _: Result<ExGuard<'_, u32, Atomic, NoPoison>, _> = s.write();
}

#[test]
fn sharex_write_then_read()
{
    let s = TestSharex::new(10);
    {
        let mut g = s.write().unwrap();
        *g = 20;
    }
    assert_eq!(*s.read().unwrap(), 20);
}

#[test]
fn sharex_multiple_concurrent_readers()
{
    let s = TestSharex::new(5);
    let r1 = s.read().unwrap();
    let r2 = s.read().unwrap();
    assert_eq!(*r1, 5);
    assert_eq!(*r2, 5);
    // Writer is blocked while readers hold the lock.
    assert!(matches!(s.try_write(), Err(TryLockError::Contention)));
    drop((r1, r2));
    assert!(s.try_write().is_ok());
}

#[test]
fn sharex_writer_blocks_readers_and_writers()
{
    let s = TestSharex::new(0);
    let w = s.write().unwrap();
    assert!(matches!(s.try_read(), Err(TryLockError::Contention)));
    assert!(matches!(s.try_write(), Err(TryLockError::Contention)));
    drop(w);
    assert!(s.try_read().is_ok());
}

#[test]
fn sharex_exchange_and_take()
{
    let s = TestSharex::new(1);
    assert_eq!(s.exchange(2).unwrap(), 1);
    assert_eq!(s.take().unwrap(), 2);
    assert_eq!(*s.read().unwrap(), 0);
}

#[test]
fn sharex_try_exchange_and_try_take()
{
    let s = TestSharex::new(10);
    assert_eq!(s.try_exchange(20).unwrap(), 10);
    assert_eq!(s.try_take().unwrap(), 20);
}

#[test]
fn sharex_is_poisoned_false_with_nopoison()
{
    let s = TestSharex::new(0);
    assert!(!s.is_poisoned());
}

#[test]
fn sharex_implements_api_traits()
{
    fn generic_read<'a, M, T, G, TryE, E>(m: &'a M) -> Result<G, E>
    where
        M: resync::api::Sharex<'a, T, G, TryE, E>,
        G: resync::api::Guard<T>,
        TryE: core::fmt::Display,
        E: core::fmt::Display,
    {
        m.read()
    }
    fn generic_write<'a, M, T, G, TryE, E>(m: &'a M) -> Result<G, E>
    where
        M: resync::api::Mutex<'a, T, G, TryE, E>,
        G: resync::api::GuardMut<T>,
        TryE: core::fmt::Display,
        E: core::fmt::Display,
    {
        m.lock()
    }
    let s = TestSharex::new(99);
    assert_eq!(*generic_read(&s).unwrap(), 99);
    assert_eq!(*generic_write(&s).unwrap(), 99);
}
