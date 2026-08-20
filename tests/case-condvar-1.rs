//! Case: Condvar event-based waiting with notify_one / notify_all.
#![cfg(all(feature = "dev", feature = "std"))]

use resync::{Condvar, Mutex};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn condvar_notify_one_wakes_waiter()
{
    let lock = Arc::new(Mutex::<bool>::new(false));
    let cvar = Arc::new(Condvar::new());

    let (l2, c2) = (Arc::clone(&lock), Arc::clone(&cvar));
    let waiter = thread::spawn(move || {
        let mut g = l2.lock().unwrap();
        // Loop guards against spurious wakeups.
        while !*g
        {
            g = c2.wait(g, &l2).unwrap();
        }
        assert!(*g);
    });

    thread::sleep(Duration::from_millis(30));
    *lock.lock().unwrap() = true;
    cvar.notify_one();

    waiter.join().unwrap();
}

#[test]
fn condvar_notify_all_wakes_everyone()
{
    let lock = Arc::new(Mutex::<u32>::new(0));
    let cvar = Arc::new(Condvar::new());
    let mut handles = vec![];

    for _ in 0..4
    {
        let (l2, c2) = (Arc::clone(&lock), Arc::clone(&cvar));
        handles.push(thread::spawn(move || {
            let mut g = l2.lock().unwrap();
            while *g == 0
            {
                g = c2.wait(g, &l2).unwrap();
            }
            *g
        }));
    }

    thread::sleep(Duration::from_millis(30));
    *lock.lock().unwrap() = 7;
    cvar.notify_all();

    for h in handles
    {
        assert_eq!(h.join().unwrap(), 7);
    }
}

#[test]
fn condvar_wait_timeout_reports_timeout()
{
    let lock = Mutex::<bool>::new(false);
    let cvar = Condvar::new();

    let g = lock.lock().unwrap();
    let (g, res) = cvar
        .wait_timeout(g, &lock, Duration::from_millis(30))
        .unwrap();
    // Nobody notified us, so it must be a timeout.
    assert!(res.timed_out());
    assert!(!*g);
}
