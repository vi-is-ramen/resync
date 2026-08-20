//! Case: Gate blocks a thread pool until the coordinator opens it.
#![cfg(feature = "dev")]

use resync::Gate;
use resync::lock::Atomic;
use resync::retry::Busy;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn gate_blocks_until_opened()
{
    let gate = Arc::new(Gate::<Atomic, Busy>::new());
    let mut handles = vec![];

    for i in 0..4u32
    {
        let g = Arc::clone(&gate);
        handles.push(thread::spawn(move || {
            g.wait().unwrap();
            i
        }));
    }

    // Workers are blocked here while we do "setup".
    thread::sleep(Duration::from_millis(30));
    gate.open();

    let mut results: Vec<u32> =
        handles.into_iter().map(|h| h.join().unwrap()).collect();
    results.sort_unstable();
    assert_eq!(results, vec![0, 1, 2, 3]);
}

#[test]
fn gate_reusable_close_then_open()
{
    let gate = Arc::new(Gate::<Atomic, Busy>::new_open());

    let g = Arc::clone(&gate);
    let h1 = thread::spawn(move || {
        g.wait().unwrap();
        1
    });
    assert_eq!(h1.join().unwrap(), 1);

    gate.close().unwrap();
    let g2 = Arc::clone(&gate);
    let h2 = thread::spawn(move || {
        g2.wait().unwrap();
        2
    });

    thread::sleep(Duration::from_millis(20));
    gate.open();
    assert_eq!(h2.join().unwrap(), 2);
}
