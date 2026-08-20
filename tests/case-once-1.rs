//! Case: Once guarantees the initializer runs exactly once across threads.

use resync::Once;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

static INIT_COUNT: AtomicUsize = AtomicUsize::new(0);

#[test]
fn once_initializes_exactly_once()
{
    let o = Arc::new(Once::<u32>::new());
    let mut handles = vec![];

    for _ in 0..8
    {
        let oc = Arc::clone(&o);
        handles.push(thread::spawn(move || {
            let v = oc
                .init(|| {
                    INIT_COUNT.fetch_add(1, Ordering::SeqCst);
                    99
                })
                .unwrap();
            assert_eq!(*v, 99);
        }));
    }

    for h in handles
    {
        h.join().unwrap();
    }

    assert_eq!(INIT_COUNT.load(Ordering::SeqCst), 1);
    assert!(o.is_completed());
    assert_eq!(*o.get().unwrap(), 99);
}
