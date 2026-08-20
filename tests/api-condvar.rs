//! Characterization: Condvar condition variable (requires std + dev).
#![cfg(all(feature = "dev", feature = "std"))]

use resync::Condvar;

#[test]
fn condvar_new_and_debug()
{
    let c = Condvar::new();
    let _ = format!("{c:?}");
}

#[test]
fn condvar_default()
{
    let c = Condvar::default();
    let _ = format!("{c:?}");
}

#[test]
fn condvar_notify_without_waiters_is_noop()
{
    let c = Condvar::new();
    c.notify_one();
    c.notify_all();
}
