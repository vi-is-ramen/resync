//! Characterization: Once one-time initialization primitive.

use resync::Once;
use resync::lock::Atomic;
use resync::poison::NoPoison;
use resync::retry::Busy;

type TestOnce<T> = Once<T, Atomic, Busy, NoPoison>;

#[allow(dead_code)]
fn _assert_default_type_params()
{
    let _: Once<u32> = Once::new();
    let _: Once<u32, Atomic> = Once::new();
    let _: Once<u32, Atomic, Busy> = Once::new();
    let _: Once<u32, Atomic, Busy, NoPoison> = Once::new();
}

#[test]
fn once_initializes_exactly_once_single_thread()
{
    let o = TestOnce::<u32>::new();
    assert!(!o.is_completed());
    assert!(o.get().is_none());

    let v = o.init(|| 42).unwrap();
    assert_eq!(*v, 42);
    assert!(o.is_completed());
    assert_eq!(*o.get().unwrap(), 42);

    // Second call must NOT run the closure.
    let v2 = o.init(|| panic!("closure must not run twice")).unwrap();
    assert_eq!(*v2, 42);
}

#[test]
fn once_default_constructor()
{
    let o: Once<u32> = Once::new();
    assert_eq!(*o.init(|| 7).unwrap(), 7);
    assert!(o.is_completed());
}

#[test]
fn once_is_poisoned_false_with_nopoison()
{
    let o = TestOnce::<u32>::new();
    assert!(!o.is_poisoned());
}

#[cfg(feature = "std")]
#[test]
fn once_poisons_when_init_panics()
{
    use resync::AcquireError;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let o = std::sync::Arc::new(Once::<u32>::new());
    let o2 = std::sync::Arc::clone(&o);
    let _ = catch_unwind(AssertUnwindSafe(move || {
        let _ = o2.init(|| -> u32 { panic!("init failed") });
    }));

    assert!(o.is_poisoned());
    // Subsequent init attempts report poisoning instead of hanging.
    match o.init(|| 1)
    {
        Err(AcquireError::Poisoned(_)) =>
        {},
        other => panic!("expected poisoned, got ok={}", other.is_ok()),
    }
}
