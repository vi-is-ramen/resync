//! Case: Semaphore caps concurrent access to a bounded resource pool.
#![cfg(feature = "dev")]

use resync::Semaphore;
use resync::lock::Atomic;
use resync::retry::Busy;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

#[test]
fn semaphore_limits_concurrency()
{
    const PERMITS: usize = 3;
    const WORKERS: usize = 8;

    let sem = Arc::new(Semaphore::<Atomic, Busy>::new(PERMITS));
    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_seen = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];
    for _ in 0..WORKERS
    {
        let s = Arc::clone(&sem);
        let cur = Arc::clone(&in_flight);
        let max = Arc::clone(&max_seen);
        handles.push(thread::spawn(move || {
            s.acquire().unwrap();
            let now = cur.fetch_add(1, Ordering::SeqCst) + 1;
            // Track the peak concurrency.
            let mut prev = max.load(Ordering::SeqCst);
            while now > prev
            {
                match max.compare_exchange(
                    prev,
                    now,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                {
                    Ok(_) => break,
                    Err(p) => prev = p,
                }
            }
            thread::sleep(Duration::from_millis(5));
            cur.fetch_sub(1, Ordering::SeqCst);
            s.release().unwrap();
        }));
    }

    for h in handles
    {
        h.join().unwrap();
    }

    // Peak concurrency must never exceed the number of permits.
    assert!(max_seen.load(Ordering::SeqCst) <= PERMITS);
    assert_eq!(sem.available_permits().unwrap(), PERMITS);
}
