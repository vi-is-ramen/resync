//! Case: Lazy static initialization shared across threads.

use resync::Lazy;
use std::thread;

fn make_vec() -> Vec<u32>
{
    vec![1, 2, 3]
}

static LAZY_VEC: Lazy<Vec<u32>> = Lazy::new(make_vec);

#[test]
fn lazy_static_initialized_once_and_shared()
{
    let mut handles = vec![];

    for _ in 0..8
    {
        handles.push(thread::spawn(|| {
            for _ in 0..200
            {
                assert_eq!(&*LAZY_VEC, &vec![1, 2, 3]);
            }
        }));
    }

    for h in handles
    {
        h.join().unwrap();
    }

    assert!(LAZY_VEC.is_initialized());
}
