//! Case: Barrier synchronizes phases and elects exactly one leader.
#![cfg(feature = "dev")]

use resync::Barrier;
use resync::retry::Busy;
use std::sync::Arc;
use std::thread;

#[test]
fn barrier_releases_all_with_one_leader()
{
    const N: usize = 5;
    let b = Arc::new(Barrier::<Busy>::new(N));
    let mut handles = vec![];

    for _ in 0..N
    {
        let bc = Arc::clone(&b);
        handles.push(thread::spawn(move || bc.wait().unwrap().is_leader()));
    }

    let leaders: Vec<bool> =
        handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(leaders.iter().filter(|&&x| x).count(), 1);
}

#[test]
fn barrier_is_reusable_across_phases()
{
    const N: usize = 3;
    let b = Arc::new(Barrier::<Busy>::new(N));
    let mut handles = vec![];

    // Two phases; each must release all threads.
    for _ in 0..2
    {
        for _ in 0..N
        {
            let bc = Arc::clone(&b);
            handles.push(thread::spawn(move || bc.wait().unwrap().is_leader()));
        }
    }

    let leaders: Vec<bool> =
        handles.into_iter().map(|h| h.join().unwrap()).collect();
    // Exactly one leader per phase -> two leaders total.
    assert_eq!(leaders.iter().filter(|&&x| x).count(), 2);
}
