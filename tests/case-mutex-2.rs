//! Case: mutex poisoning detection and data recovery across threads.

#![cfg(feature = "std")]

use resync::lock::Atomic;
use resync::poison::StdPoison;
use resync::retry::Busy;
use resync::{AcquireError, Mutex};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::thread;

// Explicitly specify StdPoison to avoid the `docsrs` cfg (enabled by the
// `__lint` feature under `--all-features`) swapping DefaultPoison to `Fake`,
// which never poisons. We also use Atomic + Busy to keep the test
// cross-platform and independent of OS-specific futex/syscall interactions
// during panic unwinding.
type PoisonableMutex = Mutex<Vec<u32>, Atomic, Busy, StdPoison>;

#[test]
fn poisoned_mutex_is_recoverable()
{
    let m = Arc::new(PoisonableMutex::new(vec![1, 2, 3]));

    let worker = Arc::clone(&m);
    let handle = thread::spawn(move || {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let mut g = worker.lock().unwrap();
            g.push(4);
            panic!("worker crashed mid-critical-section");
        }));
    });
    handle.join().unwrap();

    assert!(m.is_poisoned());

    match m.lock()
    {
        Err(AcquireError::Poisoned(pe)) =>
        {
            let mut g = pe.into_inner();
            // Data reflects the partial write performed before the panic.
            assert_eq!(*g, vec![1, 2, 3, 4]);
            g.clear(); // manually repair
        },
        Ok(_) => panic!("expected the mutex to be poisoned"),
        Err(e) => panic!("unexpected acquire error: {e:?}"),
    }

    unsafe { m.clear_poison() };
    assert!(!m.is_poisoned());
    assert!(m.lock().unwrap().is_empty());
}
