//! Characterization: Barrier primitive.
#![cfg(feature = "dev")]

use resync::Barrier;
use resync::retry::Busy;

#[test]
fn barrier_single_thread_is_leader()
{
    let b = Barrier::<Busy>::new(1);
    assert!(b.wait().unwrap().is_leader());
}

#[test]
fn barrier_is_reusable()
{
    let b = Barrier::<Busy>::new(1);
    assert!(b.wait().unwrap().is_leader());
    assert!(b.wait().unwrap().is_leader());
    assert!(b.wait().unwrap().is_leader());
}

#[test]
fn barrier_with_retry_constructor()
{
    let b = Barrier::with_retry(1, Busy);
    assert!(b.wait().unwrap().is_leader());
}

#[test]
fn barrier_debug()
{
    let b = Barrier::<Busy>::new(2);
    let _ = format!("{b:?}");
}

#[test]
#[should_panic]
fn barrier_zero_threads_panics()
{
    let _ = Barrier::<Busy>::new(0);
}

#[test]
#[should_panic]
fn barrier_with_retry_zero_panics()
{
    let _ = Barrier::with_retry(0, Busy);
}
