//! Case: Sharex read-write semantics under concurrency.

use resync::Sharex;
use resync::lock::Atomic;
use resync::poison::NoPoison;
use resync::retry::Busy;
use std::sync::Arc;
use std::thread;

type RwSpin = Sharex<u64, Atomic, Busy, NoPoison>;

#[test]
fn sharex_many_concurrent_readers()
{
    let data = Arc::new(RwSpin::new(42));
    let mut handles = vec![];

    for _ in 0..8
    {
        let d = Arc::clone(&data);
        handles.push(thread::spawn(move || {
            for _ in 0..500
            {
                assert_eq!(*d.read().unwrap(), 42);
            }
        }));
    }

    for h in handles
    {
        h.join().unwrap();
    }
}

#[test]
fn sharex_writers_are_exclusive()
{
    let data = Arc::new(RwSpin::new(0));
    let mut handles = vec![];

    for _ in 0..4
    {
        let d = Arc::clone(&data);
        handles.push(thread::spawn(move || {
            for _ in 0..250
            {
                *d.write().unwrap() += 1;
            }
        }));
    }

    for h in handles
    {
        h.join().unwrap();
    }

    assert_eq!(*data.read().unwrap(), 1000);
}
