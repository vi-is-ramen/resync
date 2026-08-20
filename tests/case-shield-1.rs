//! Case: Shield prevents writer starvation under a continuous reader stream.

use resync::Sharex;
use resync::lock::{Atomic, Shield};
use resync::poison::NoPoison;
use resync::retry::Busy;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

type FairRw = Sharex<u64, Shield<Atomic>, Busy, NoPoison>;

#[test]
fn shield_writer_is_not_starved_by_readers()
{
    let data = Arc::new(FairRw::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let mut readers = vec![];

    // Continuous reader stream.
    for _ in 0..4
    {
        let d = Arc::clone(&data);
        let s = Arc::clone(&stop);
        readers.push(thread::spawn(move || {
            while !s.load(Ordering::Relaxed)
            {
                let _ = *d.read().unwrap();
            }
        }));
    }

    // Let readers get going, then a writer must still be able to acquire.
    thread::sleep(Duration::from_millis(20));
    *data.write().unwrap() = 42;

    stop.store(true, Ordering::Relaxed);
    for r in readers
    {
        r.join().unwrap();
    }

    assert_eq!(*data.read().unwrap(), 42);
}
