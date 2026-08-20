//! Characterization: api::Guard / api::GuardMut traits and their impls.

use resync::api::{Guard, GuardMut};

#[allow(dead_code)]
fn _assert_guard<T, G: Guard<T>>(_g: &G) {}

#[allow(dead_code)]
fn _assert_guard_mut<T, G: GuardMut<T>>(_g: &G) {}

#[test]
fn resync_exguard_implements_guard_and_guardmut()
{
    use resync::Mutex;
    use resync::lock::Atomic;
    use resync::poison::NoPoison;
    use resync::retry::Busy;
    let m = Mutex::<i32, Atomic, Busy, NoPoison>::new(1);
    let g = m.lock().unwrap();
    _assert_guard(&g);
    _assert_guard_mut(&g);
}

#[test]
fn resync_shguard_implements_guard()
{
    use resync::Sharex;
    use resync::lock::Atomic;
    use resync::poison::NoPoison;
    use resync::retry::Busy;
    let s = Sharex::<i32, Atomic, Busy, NoPoison>::new(1);
    let g = s.read().unwrap();
    _assert_guard(&g);
}

#[cfg(feature = "std")]
#[test]
fn std_guards_implement_traits()
{
    use std::sync::{Mutex, RwLock};
    let m = Mutex::new(1);
    let g = m.lock().unwrap();
    _assert_guard(&g);
    _assert_guard_mut(&g);
    drop(g);

    let rw = RwLock::new(2);
    let rg = rw.read().unwrap();
    _assert_guard(&rg);
    drop(rg);
    #[allow(clippy::readonly_write_lock)]
    let wg = rw.write().unwrap();
    _assert_guard(&wg);
    _assert_guard_mut(&wg);
}
