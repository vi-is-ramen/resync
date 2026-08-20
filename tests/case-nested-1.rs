//! Case: Nested composite lock enforces a fixed acquisition order.

use resync::Mutex;
use resync::lock::{Atomic, Nested};
use resync::poison::NoPoison;
use resync::retry::Busy;
use std::sync::Arc;
use std::thread;

type NestedMutex = Mutex<u32, Nested<Atomic, Atomic>, Busy, NoPoison>;

#[test]
fn nested_mutex_basic_cycle()
{
    let m = NestedMutex::new(0);
    *m.lock().unwrap() += 1;
    assert_eq!(*m.lock().unwrap(), 1);
}

#[test]
fn nested_mutex_under_contention()
{
    let m = Arc::new(NestedMutex::new(0));
    let mut handles = vec![];

    for _ in 0..4
    {
        let mc = Arc::clone(&m);
        handles.push(thread::spawn(move || {
            for _ in 0..100
            {
                *mc.lock().unwrap() += 1;
            }
        }));
    }

    for h in handles
    {
        h.join().unwrap();
    }

    assert_eq!(*m.lock().unwrap(), 400);
}
