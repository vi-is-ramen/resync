#![allow(clippy::all, missing_docs)]

mod common;

use common::{THREADS, iters_to_usize, measure_threads};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::Arc;

type OsMutex<T> =
    resync::Mutex<T, common::OsLock, common::OsRetry, common::OsPoison>;

type SpinMutex<T> =
    resync::Mutex<T, common::SpinLock, common::SpinRetry, common::SpinPoison>;

fn uncontended(c: &mut Criterion)
{
    let mut group = c.benchmark_group("mutex/uncontended");

    group.bench_function("std_lock", |b| {
        let m = std::sync::Mutex::new(0u64);

        b.iter(|| {
            let mut g = m.lock().unwrap();
            *g += 1;
            black_box(*g);
        });
    });

    group.bench_function("resync_os_lock", |b| {
        let m = OsMutex::<u64>::new(0);

        b.iter(|| {
            let mut g = m.lock().unwrap();
            *g += 1;
            black_box(*g);
        });
    });

    group.bench_function("resync_spin_lock", |b| {
        let m = SpinMutex::<u64>::new(0);

        b.iter(|| {
            let mut g = m.lock().unwrap();
            *g += 1;
            black_box(*g);
        });
    });

    group.bench_function("std_try_lock", |b| {
        let m = std::sync::Mutex::new(0u64);

        b.iter(|| {
            let mut g = m.try_lock().unwrap();
            *g += 1;
            black_box(*g);
        });
    });

    group.bench_function("resync_os_try_lock", |b| {
        let m = OsMutex::<u64>::new(0);

        b.iter(|| {
            let mut g = m.try_lock().unwrap();
            *g += 1;
            black_box(*g);
        });
    });

    group.bench_function("resync_spin_try_lock", |b| {
        let m = SpinMutex::<u64>::new(0);

        b.iter(|| {
            let mut g = m.try_lock().unwrap();
            *g += 1;
            black_box(*g);
        });
    });
}

fn contended(c: &mut Criterion)
{
    let mut group = c.benchmark_group("mutex/contended");
    group.sample_size(20);

    group.bench_function("std_lock", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let m = Arc::new(std::sync::Mutex::new(0u64));

            measure_threads(THREADS, total, || {
                let m = Arc::clone(&m);

                move |n| {
                    for _ in 0..n
                    {
                        let mut g = m.lock().unwrap();
                        *g += 1;
                        black_box(*g);
                    }
                }
            })
        });
    });

    group.bench_function("resync_os_lock", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let m = Arc::new(OsMutex::<u64>::new(0));

            measure_threads(THREADS, total, || {
                let m = Arc::clone(&m);

                move |n| {
                    for _ in 0..n
                    {
                        let mut g = m.lock().unwrap();
                        *g += 1;
                        black_box(*g);
                    }
                }
            })
        });
    });

    group.bench_function("resync_spin_lock", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let m = Arc::new(SpinMutex::<u64>::new(0));

            measure_threads(THREADS, total, || {
                let m = Arc::clone(&m);

                move |n| {
                    for _ in 0..n
                    {
                        let mut g = m.lock().unwrap();
                        *g += 1;
                        black_box(*g);
                    }
                }
            })
        });
    });
}

criterion_group!(mutex, uncontended, contended);
criterion_main!(mutex);
