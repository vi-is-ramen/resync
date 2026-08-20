//! # Mutex
//!
//! Simple test for mutex under 64 conccuring threads.

use resync::{Lazy, Mutex};
use std::thread::spawn;

static M: Lazy<Mutex<u32>> = Lazy::new(Mutex::default);

#[test]
fn main()
{
    let pool = {
        let mut pool = vec![];

        for _ in 0..64
        {
            pool.push(spawn(|| {
                let mut m = M.lock().expect("Failed to lock the Mutex");

                *m += 2;
                *m -= 1;
            }));
        }
        pool
    };

    for jh in pool
    {
        let _ = jh.join();
    }

    assert_eq!(
        *M.lock().expect("Mutex lock finished at checking step"),
        64,
        "Final mutex value has invalid value"
    );
}
