//! .

// //! # Mutex
// //!
// //! Simple test for mutex under 64 conccuring threads.

// use std::sync::LazyLock;
// use std::thread::spawn;

// macro_rules!parallel{{$n:expr;$($tt:tt)*}=>{{let mut pool=vec![];for _ in
// 0..$n {pool.push(spawn(||{$($tt)*}));}pool}}}

// static M: LazyLock<resync::Mutex<u32>> =
// LazyLock::new(resync::Mutex::default);

// fn main()
// {
//     let pool = parallel! {
//         64;

//         let mut m = M.lock().expect("Mutex lock failed");

//         *m += 2;
//         *m -= 1;
//     };

//     for jh in pool
//     {
//         let _ = jh.join();
//     }

//     assert_eq!(
//         *M.lock().expect("Mutex lock finished at checking step"),
//         64,
//         "Final mutex value has invalid value"
//     );
// }

fn main() {}
