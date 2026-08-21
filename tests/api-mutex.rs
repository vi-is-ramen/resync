//! Characterization: Mutex primitive + ExGuard (API surface + behavior).

use resync::api::{LockPolicy, PoisonPolicy, RetryPolicy};
use resync::lock::Atomic;
use resync::poison::NoPoison;
use resync::retry::Busy;
use resync::{ExGuard, Mutex, TryLockError};

type TestMutex = Mutex<u32, Atomic, Busy, NoPoison>;

// --- Compile-time: default type parameters are stable ---
#[allow(dead_code)]
fn _assert_default_type_params()
{
    let _: Mutex<u32> = Mutex::new(0);
    let _: Mutex<u32, Atomic> = Mutex::new(0);
    let _: Mutex<u32, Atomic, Busy> = Mutex::new(0);
    let _: Mutex<u32, Atomic, Busy, NoPoison> = Mutex::new(0);
}

// --- Compile-time: generic constructor bounds ---
#[allow(dead_code)]
fn _assert_generic_bounds<T, L, R, P>()
where
    T: Default,
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy + Default,
{
    let _: Mutex<T, L, R, P> = Mutex::new(T::default());
}

// --- Compile-time: guard type is ExGuard ---
#[allow(dead_code)]
fn _assert_guard_type(m: &TestMutex)
{
    let _: Result<ExGuard<'_, u32, Atomic, NoPoison>, _> = m.lock();
}

#[test]
fn mutex_lock_deref_derefmut()
{
    let m = TestMutex::new(42);
    let mut g = m.lock().unwrap();
    assert_eq!(*g, 42);
    *g += 1;
    assert_eq!(*g, 43);
}

#[test]
fn mutex_try_lock_contention_and_release()
{
    let m = TestMutex::new(0);
    let g = m.lock().unwrap();
    assert!(matches!(m.try_lock(), Err(TryLockError::Contention)));
    drop(g);
    assert!(m.try_lock().is_ok());
}

#[test]
fn mutex_default_yields_default_value()
{
    let m = TestMutex::default();
    let g = m.lock().unwrap();
    assert_eq!(*g, 0);
}

#[test]
fn mutex_exchange_and_take()
{
    let m = TestMutex::new(1);
    assert_eq!(m.exchange(2).unwrap(), 1);
    assert_eq!(m.take().unwrap(), 2);
    let g = m.lock().unwrap();
    assert_eq!(*g, 0);
}

#[test]
fn mutex_try_exchange_and_try_take()
{
    let m = TestMutex::new(5);
    assert_eq!(m.try_exchange(6).unwrap(), 5);
    assert_eq!(m.try_take().unwrap(), 6);
}

#[test]
fn mutex_guard_exchange_and_take_consume_guard()
{
    let m = TestMutex::new(1);

    // exchange consumes the guard and releases the lock
    let g = m.lock().unwrap();
    assert_eq!(g.exchange(9), 1);

    // verify the value was exchanged
    {
        let g2 = m.lock().unwrap();
        assert_eq!(*g2, 9);
    } // g2 dropped here, lock released

    // take consumes the guard and releases the lock
    let g3 = m.lock().unwrap();
    assert_eq!(g3.take(), 9);

    // verify the value was taken (replaced with Default)
    {
        let g4 = m.lock().unwrap();
        assert_eq!(*g4, 0);
    }
}

#[test]
fn mutex_debug_renders_inner_value()
{
    let m = TestMutex::new(7);
    assert!(format!("{m:?}").contains('7'));
}

#[test]
fn mutex_is_poisoned_false_with_nopoison()
{
    let m = TestMutex::new(0);
    assert!(!m.is_poisoned());
}

#[test]
fn mutex_implements_api_mutex_trait()
{
    fn generic_lock<'a, M, T, G, TryE, E>(m: &'a M) -> Result<G, E>
    where
        M: resync::api::Mutex<'a, T, G, TryE, E>,
        G: resync::api::GuardMut<T>,
        TryE: core::fmt::Display,
        E: core::fmt::Display,
    {
        m.lock()
    }
    let m = TestMutex::new(11);
    let g = generic_lock(&m).unwrap();
    assert_eq!(*g, 11);
}

// --- Poisoning behavior (requires std + StdPoison) ---
#[cfg(feature = "std")]
mod poisoning
{
    use resync::lock::Atomic;
    use resync::poison::StdPoison;
    use resync::retry::Busy;
    use resync::{AcquireError, Mutex, TryLockError};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;

    // Use Atomic + Busy + StdPoison to avoid Futex syscall interactions
    // with catch_unwind / panic unwinding.
    type PoisonableMutex = Mutex<u32, Atomic, Busy, StdPoison>;

    fn poison(m: Arc<PoisonableMutex>, value: u32)
    {
        let _ = catch_unwind(AssertUnwindSafe(move || {
            let mut g = m.lock().unwrap();
            *g = value;
            panic!("intentional");
        }));
    }

    #[test]
    fn lock_poisons_on_panic()
    {
        let m = Arc::new(PoisonableMutex::new(0));
        poison(Arc::clone(&m), 42);
        assert!(m.is_poisoned());
    }

    #[test]
    fn poisoned_lock_returns_recovery_guard()
    {
        let m = Arc::new(PoisonableMutex::new(0));
        poison(Arc::clone(&m), 42);
        match m.lock()
        {
            Err(AcquireError::Poisoned(pe)) =>
            {
                let mut g = pe.into_inner();
                assert_eq!(*g, 42);
                *g = 0;
            },
            Ok(_) => panic!("expected poisoned error"),
            Err(e) => panic!("unexpected error: {e:?}"),
        }
        unsafe { m.clear_poison() };
        assert!(!m.is_poisoned());
        assert_eq!(*m.lock().unwrap(), 0);
    }

    #[test]
    fn poisoned_try_lock_returns_recovery_guard()
    {
        let m = Arc::new(PoisonableMutex::new(0));
        poison(Arc::clone(&m), 7);
        match m.try_lock()
        {
            Err(TryLockError::Poisoned(pe)) =>
            {
                assert_eq!(*pe.into_inner(), 7);
            },
            Ok(_) => panic!("expected poisoned try_lock error"),
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }
}
