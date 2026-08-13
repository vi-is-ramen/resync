//! # Mutex
//!
//! Simple test for mutex under 64 conccuring threads.

use resync::*;
use resync_tests::*;
use std::sync::LazyLock;
use std::thread::spawn;

static M: LazyLock<Mutex<u32>> = LazyLock::new(Mutex::default);

fn main()
{
    let pool = parallel! {
        in 64 =>

        let mut m = M.lock().expect("Mutex lock failed");

        *m += 2;
        *m -= 1;
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
