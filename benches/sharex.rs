#![allow(clippy::all, missing_docs)]

mod common;

use common::{THREADS, iters_to_usize, measure_threads};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::Arc;

type OsRwLock<T> = resync::Sharex<
    T,
    resync::lock::Shield<common::OsLock>,
    common::OsRetry,
    common::OsPoison,
>;

type SpinRwLock<T> =
    resync::Sharex<T, common::SpinLock, common::SpinRetry, common::SpinPoison>;

fn uncontended(c: &mut Criterion)
{
    let mut group = c.benchmark_group("rwlock/uncontended");

    group.bench_function("std_read", |b| {
        let rw = std::sync::RwLock::new(42u64);

        b.iter(|| {
            let g = rw.read().unwrap();
            black_box(*g);
        });
    });

    group.bench_function("resync_os_read", |b| {
        let rw = OsRwLock::<u64>::new(42);

        b.iter(|| {
            let g = rw.read().unwrap();
            black_box(*g);
        });
    });

    group.bench_function("resync_spin_read", |b| {
        let rw = SpinRwLock::<u64>::new(42);

        b.iter(|| {
            let g = rw.read().unwrap();
            black_box(*g);
        });
    });

    group.bench_function("std_write", |b| {
        let rw = std::sync::RwLock::new(0u64);

        b.iter(|| {
            let mut g = rw.write().unwrap();
            *g += 1;
            black_box(*g);
        });
    });

    group.bench_function("resync_os_write", |b| {
        let rw = OsRwLock::<u64>::new(0);

        b.iter(|| {
            let mut g = rw.write().unwrap();
            *g += 1;
            black_box(*g);
        });
    });

    group.bench_function("resync_spin_write", |b| {
        let rw = SpinRwLock::<u64>::new(0);

        b.iter(|| {
            let mut g = rw.write().unwrap();
            *g += 1;
            black_box(*g);
        });
    });

    group.bench_function("std_try_read", |b| {
        let rw = std::sync::RwLock::new(42u64);

        b.iter(|| {
            let g = rw.try_read().unwrap();
            black_box(*g);
        });
    });

    group.bench_function("resync_os_try_read", |b| {
        let rw = OsRwLock::<u64>::new(42);

        b.iter(|| {
            let g = rw.try_read().unwrap();
            black_box(*g);
        });
    });

    group.bench_function("resync_spin_try_read", |b| {
        let rw = SpinRwLock::<u64>::new(42);

        b.iter(|| {
            let g = rw.try_read().unwrap();
            black_box(*g);
        });
    });

    group.bench_function("std_try_write", |b| {
        let rw = std::sync::RwLock::new(0u64);

        b.iter(|| {
            let mut g = rw.try_write().unwrap();
            *g += 1;
            black_box(*g);
        });
    });

    group.bench_function("resync_os_try_write", |b| {
        let rw = OsRwLock::<u64>::new(0);

        b.iter(|| {
            let mut g = rw.try_write().unwrap();
            *g += 1;
            black_box(*g);
        });
    });

    group.bench_function("resync_spin_try_write", |b| {
        let rw = SpinRwLock::<u64>::new(0);

        b.iter(|| {
            let mut g = rw.try_write().unwrap();
            *g += 1;
            black_box(*g);
        });
    });
}

fn contended_read(c: &mut Criterion)
{
    let mut group = c.benchmark_group("rwlock/contended_read");
    group.sample_size(20);

    group.bench_function("std_read", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let rw = Arc::new(std::sync::RwLock::new(42u64));

            measure_threads(THREADS, total, || {
                let rw = Arc::clone(&rw);

                move |n| {
                    for _ in 0..n
                    {
                        let g = rw.read().unwrap();
                        black_box(*g);
                    }
                }
            })
        });
    });

    group.bench_function("resync_os_read", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let rw = Arc::new(OsRwLock::<u64>::new(42));

            measure_threads(THREADS, total, || {
                let rw = Arc::clone(&rw);

                move |n| {
                    for _ in 0..n
                    {
                        let g = rw.read().unwrap();
                        black_box(*g);
                    }
                }
            })
        });
    });

    group.bench_function("resync_spin_read", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let rw = Arc::new(SpinRwLock::<u64>::new(42));

            measure_threads(THREADS, total, || {
                let rw = Arc::clone(&rw);

                move |n| {
                    for _ in 0..n
                    {
                        let g = rw.read().unwrap();
                        black_box(*g);
                    }
                }
            })
        });
    });
}

fn contended_write(c: &mut Criterion)
{
    let mut group = c.benchmark_group("rwlock/contended_write");
    group.sample_size(20);

    group.bench_function("std_write", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let rw = Arc::new(std::sync::RwLock::new(0u64));

            measure_threads(THREADS, total, || {
                let rw = Arc::clone(&rw);

                move |n| {
                    for _ in 0..n
                    {
                        let mut g = rw.write().unwrap();
                        *g += 1;
                        black_box(*g);
                    }
                }
            })
        });
    });

    group.bench_function("resync_os_write", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let rw = Arc::new(OsRwLock::<u64>::new(0));

            measure_threads(THREADS, total, || {
                let rw = Arc::clone(&rw);

                move |n| {
                    for _ in 0..n
                    {
                        let mut g = rw.write().unwrap();
                        *g += 1;
                        black_box(*g);
                    }
                }
            })
        });
    });

    group.bench_function("resync_spin_write", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let rw = Arc::new(SpinRwLock::<u64>::new(0));

            measure_threads(THREADS, total, || {
                let rw = Arc::clone(&rw);

                move |n| {
                    for _ in 0..n
                    {
                        let mut g = rw.write().unwrap();
                        *g += 1;
                        black_box(*g);
                    }
                }
            })
        });
    });
}

criterion_group!(sharex, uncontended, contended_read, contended_write);
criterion_main!(sharex);
