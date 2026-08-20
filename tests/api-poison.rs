//! Characterization: poison policies (NoPoison, StdPoison, DefaultPoison).

use resync::poison::NoPoison;
use resync::traits::PoisonPolicy;

#[test]
fn no_poison_never_poisons()
{
    let p = NoPoison;
    assert!(!p.is_poisoned());
    p.on_drop();
    assert!(!p.is_poisoned());
    unsafe { p.clear_poison() };
    assert!(!p.is_poisoned());
}

#[test]
fn no_poison_default_clone_copy_debug()
{
    #[allow(clippy::default_constructed_unit_structs)]
    let a = NoPoison::default();
    let b = a; // Copy
    #[allow(clippy::clone_on_copy)]
    let c = a.clone(); // Clone
    let _ = format!("{a:?}{b:?}{c:?}");
    assert!(!a.is_poisoned());
}

#[test]
fn default_poison_is_usable()
{
    // DefaultPoison resolves to StdPoison (std) or NoPoison (no_std).
    let p = <resync::poison::DefaultPoison as Default>::default();
    let _ = p.is_poisoned();
}

#[cfg(feature = "std")]
mod std_poison
{
    use resync::poison::StdPoison;
    use resync::traits::PoisonPolicy;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;

    #[test]
    fn std_poison_marks_on_panicking_drop()
    {
        let p = Arc::new(StdPoison::default());
        assert!(!p.is_poisoned());

        let p2 = Arc::clone(&p);
        let _ = catch_unwind(AssertUnwindSafe(move || {
            struct OnDropGuard<'a>(&'a StdPoison);
            impl<'a> Drop for OnDropGuard<'a>
            {
                fn drop(&mut self)
                {
                    self.0.on_drop();
                }
            }
            // Dropped during unwinding while thread::panicking() == true.
            let _g = OnDropGuard(&p2);
            panic!("trigger");
        }));

        assert!(p.is_poisoned());
        unsafe { p.clear_poison() };
        assert!(!p.is_poisoned());
    }

    #[test]
    fn std_poison_not_marked_on_normal_drop()
    {
        let p = StdPoison::default();
        p.on_drop(); // not panicking
        assert!(!p.is_poisoned());
    }
}
