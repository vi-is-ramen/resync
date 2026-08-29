//! Case: Retaint provides reentrancy without breaking mutual exclusion.
//!
//! These tests verify the runtime correctness of the reentrant wrapper:
//! - The same thread may lock reentrantly without deadlocking.
//! - Mutual exclusion between different threads is still enforced.
//! - The inner lock is only released when the guard count reaches zero.
#![cfg(all(feature = "dev", feature = "std"))]

use resync::Mutex;
use resync::lock::{Atomic, Retaint};
use resync::poison::NoPoison;
use resync::retry::Busy;
use std::sync::Arc;
use std::thread;

type ReentrantMutex = Mutex<u32, Retaint<Atomic>, Busy, NoPoison>;

#[test]
fn reentrant_mutex_does_not_deadlock_on_nested_locking()
{
    let m = ReentrantMutex::new(0);

    let g1 = m.lock().unwrap();
    // Locking again from the same thread must NOT deadlock.
    let mut g2 = m.lock().unwrap();
    *g2 += 1;
    // A third nesting level.
    let mut g3 = m.lock().unwrap();
    *g3 += 1;
    drop(g3);
    drop(g2);

    assert_eq!(*g1, 2);
    drop(g1);

    // After all guards are dropped the value is observable again.
    assert_eq!(*m.lock().unwrap(), 2);
}

#[test]
fn reentrant_mutex_preserves_mutual_exclusion_across_threads()
{
    const THREADS: usize = 8;
    const ITERS: usize = 500;

    let m = Arc::new(ReentrantMutex::new(0));
    let mut handles = Vec::with_capacity(THREADS);

    for _ in 0..THREADS
    {
        let mc = Arc::clone(&m);
        handles.push(thread::spawn(move || {
            for _ in 0..ITERS
            {
                // Each iteration takes the lock twice (reentrantly) and
                // increments once per level.
                let mut g1 = mc.lock().unwrap();
                let mut g2 = mc.lock().unwrap();
                *g2 += 1;
                drop(g2);
                *g1 += 1;
                drop(g1);
            }
        }));
    }

    for h in handles
    {
        h.join().unwrap();
    }

    // Each iteration adds exactly 2; any lost update would break this.
    assert_eq!(*m.lock().unwrap(), (THREADS * ITERS * 2) as u32);
}

#[test]
fn inner_lock_is_released_only_after_last_guard()
{
    use resync::LockStatus;
    use resync::api::LockPolicy;

    let lock = Arc::new(Retaint::<Atomic>::default());

    // Main thread acquires twice.
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));
    assert_eq!(unsafe { lock.try_lock(0) }.unwrap(), LockStatus::Done(()));

    // While held, another thread cannot acquire.
    let l2 = Arc::clone(&lock);
    let probe = thread::spawn(move || unsafe { l2.try_lock(0) }.unwrap());
    assert_eq!(probe.join().unwrap(), LockStatus::Fail);

    // Release one level: still held.
    unsafe { lock.free(&()) };
    let l3 = Arc::clone(&lock);
    let probe = thread::spawn(move || unsafe { l3.try_lock(0) }.unwrap());
    assert_eq!(probe.join().unwrap(), LockStatus::Fail);

    // Release the last level: now a different thread can acquire.
    unsafe { lock.free(&()) };
    let l4 = Arc::clone(&lock);
    let probe = thread::spawn(move || {
        let status = unsafe { l4.try_lock(0) }.unwrap();
        if let LockStatus::Done(meta) = &status
        {
            unsafe { l4.free(meta) };
        }
        status
    });
    assert_eq!(probe.join().unwrap(), LockStatus::Done(()));
}
