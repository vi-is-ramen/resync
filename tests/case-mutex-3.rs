//! Case: mutex correctness under high contention (spin backend,
//! no_std-friendly).

use resync::Mutex;
use resync::lock::Atomic;
use resync::poison::NoPoison;
use resync::retry::Busy;
use std::sync::Arc;
use std::thread;

type SpinMutex = Mutex<u64, Atomic, Busy, NoPoison>;

#[test]
fn mutex_increments_under_contention()
{
    const THREADS: usize = 8;
    const ITERS: usize = 1000;

    let m = Arc::new(SpinMutex::new(0));
    let mut handles = Vec::with_capacity(THREADS);

    for _ in 0..THREADS
    {
        let mc = Arc::clone(&m);
        handles.push(thread::spawn(move || {
            for _ in 0..ITERS
            {
                *mc.lock().unwrap() += 1;
            }
        }));
    }

    for h in handles
    {
        h.join().unwrap();
    }

    assert_eq!(*m.lock().unwrap(), (THREADS * ITERS) as u64);
}
