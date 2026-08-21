#![allow(clippy::all, missing_docs, unused)]

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

pub const THREADS: usize = 8;

#[cfg(target_os = "linux")]
pub type OsLock = resync::lock::Futex;

#[cfg(target_os = "windows")]
pub type OsLock = resync::lock::Srw;

#[cfg(target_os = "macos")]
pub type OsLock = resync::lock::Rwl;

#[cfg(not(any(
    target_os = "linux",
    target_os = "windows",
    target_os = "macos"
)))]
pub type OsLock = resync::lock::Atomic;

pub type OsRetry = resync::retry::Yield;
pub type OsPoison = resync::poison::StdPoison;

pub type SpinLock = resync::lock::Atomic;
pub type SpinRetry = resync::retry::Busy;
pub type SpinPoison = resync::poison::NoPoison;

pub fn iters_to_usize(iters: u64) -> usize
{
    usize::try_from(iters).unwrap_or(usize::MAX)
}

pub fn measure_threads<F, W>(
    threads: usize,
    total_iters: usize,
    mut make_worker: F,
) -> Duration
where
    F: FnMut() -> W,
    W: FnOnce(usize) + Send + 'static,
{
    let start = Arc::new(Barrier::new(threads + 1));
    let per_thread = total_iters.div_ceil(threads);
    let mut handles = Vec::with_capacity(threads);

    for _ in 0..threads
    {
        let start = Arc::clone(&start);
        let worker = make_worker();

        handles.push(thread::spawn(move || {
            let _ = start.wait();
            worker(per_thread);
        }));
    }

    let _ = start.wait();
    let started = Instant::now();

    for handle in handles
    {
        handle.join().unwrap();
    }

    started.elapsed()
}

pub fn measure_two_roles<A, B, FA, FB>(
    iters: usize,
    mut make_first: FA,
    mut make_second: FB,
) -> Duration
where
    A: FnOnce(usize) + Send + 'static,
    B: FnOnce(usize) + Send + 'static,
    FA: FnMut() -> A,
    FB: FnMut() -> B,
{
    let start = Arc::new(Barrier::new(3));

    let first = {
        let start = Arc::clone(&start);
        let worker = make_first();

        thread::spawn(move || {
            let _ = start.wait();
            worker(iters);
        })
    };

    let second = {
        let start = Arc::clone(&start);
        let worker = make_second();

        thread::spawn(move || {
            let _ = start.wait();
            worker(iters);
        })
    };

    let _ = start.wait();
    let started = Instant::now();

    first.join().unwrap();
    second.join().unwrap();

    started.elapsed()
}
