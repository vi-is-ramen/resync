//! Characterization: Lazy initialization primitive.

use resync::Lazy;

fn make_u32() -> u32
{
    42
}

#[test]
fn lazy_inits_on_force()
{
    let l = Lazy::<u32>::new(make_u32);
    assert!(!l.is_initialized());
    assert!(!l.is_poisoned());
    assert_eq!(*Lazy::force(&l), 42);
    assert!(l.is_initialized());
    assert!(!l.is_poisoned());
}

#[test]
fn lazy_deref_forces_init()
{
    let l = Lazy::<u32>::new(|| 7u32);
    assert_eq!(*l, 7);
    assert!(l.is_initialized());
    assert!(!l.is_poisoned());
}

fn make_string() -> String
{
    String::from("hello")
}

static LAZY_STR: Lazy<String> = Lazy::new(make_string);

#[test]
fn lazy_usable_in_static()
{
    assert_eq!(&*LAZY_STR, "hello");
    assert!(LAZY_STR.is_initialized());
}

#[cfg(feature = "std")]
#[test]
fn lazy_poisons_when_init_panics()
{
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;

    let l = Arc::new(Lazy::<u32, _>::new(|| -> u32 { panic!("boom") }));
    let l2 = Arc::clone(&l);
    let _ = catch_unwind(AssertUnwindSafe(move || {
        let _ = Lazy::force(&l2);
    }));

    assert!(l.is_poisoned());
    assert!(!l.is_initialized());
}

#[cfg(feature = "std")]
#[test]
#[should_panic(expected = "Lazy initialization previously panicked")]
fn lazy_force_panics_after_poisoning()
{
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let l = Lazy::<u32, _>::new(|| -> u32 { panic!("boom") });
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _ = Lazy::force(&l);
    }));
    // Second access must panic because the Lazy is poisoned.
    let _ = Lazy::force(&l);
}

#[cfg(feature = "std")]
#[test]
#[should_panic(expected = "Lazy initialization previously panicked")]
fn lazy_deref_panics_after_poisoning()
{
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let l = Lazy::<u32, _>::new(|| -> u32 { panic!("boom") });
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _: &u32 = &l;
    }));
    // Deref must panic because the Lazy is poisoned.
    let _: &u32 = &l;
}
