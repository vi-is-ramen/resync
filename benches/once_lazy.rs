#![allow(clippy::all, missing_docs)]

mod common;

use common::{THREADS, iters_to_usize, measure_threads};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::Arc;

fn make_42() -> u64
{
    42
}

type OsOnce =
    resync::Once<u64, common::OsLock, common::OsRetry, common::OsPoison>;

type OsLazy = resync::Lazy<
    u64,
    fn() -> u64,
    common::OsLock,
    common::OsRetry,
    common::OsPoison,
>;

fn initialized_access(c: &mut Criterion)
{
    let mut group = c.benchmark_group("once_lazy/initialized_access");

    group.bench_function("std_once_lock_get", |b| {
        let o = std::sync::OnceLock::<u64>::new();
        o.set(42).unwrap();

        b.iter(|| {
            black_box(o.get().copied().unwrap());
        });
    });

    group.bench_function("resync_once_get", |b| {
        let o = OsOnce::new();
        o.init(make_42).unwrap();

        b.iter(|| {
            black_box(*o.get().unwrap());
        });
    });

    group.bench_function("std_lazy_lock_deref", |b| {
        let l = std::sync::LazyLock::new(make_42);
        let _ = *l;

        b.iter(|| {
            black_box(*l);
        });
    });

    group.bench_function("resync_lazy_deref", |b| {
        let l = OsLazy::new(make_42);
        let _ = *l;

        b.iter(|| {
            black_box(*l);
        });
    });
}

fn initialized_contention(c: &mut Criterion)
{
    let mut group = c.benchmark_group("once_lazy/initialized_contention");
    group.sample_size(20);

    group.bench_function("std_once_lock_get", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let o = Arc::new(std::sync::OnceLock::<u64>::new());
            o.set(42).unwrap();

            measure_threads(THREADS, total, || {
                let o = Arc::clone(&o);

                move |n| {
                    for _ in 0..n
                    {
                        black_box(*o.get().unwrap());
                    }
                }
            })
        });
    });

    group.bench_function("resync_once_get", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let o = Arc::new(OsOnce::new());
            o.init(make_42).unwrap();

            measure_threads(THREADS, total, || {
                let o = Arc::clone(&o);

                move |n| {
                    for _ in 0..n
                    {
                        black_box(*o.get().unwrap());
                    }
                }
            })
        });
    });

    group.bench_function("resync_lazy_deref", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let l = Arc::new(OsLazy::new(make_42));
            let _ = *l;

            measure_threads(THREADS, total, || {
                let l = Arc::clone(&l);

                move |n| {
                    for _ in 0..n
                    {
                        black_box(&*l);
                    }
                }
            })
        });
    });
}

fn first_touch_contention(c: &mut Criterion)
{
    let mut group = c.benchmark_group("once_lazy/first_touch_contention");
    group.sample_size(20);

    group.bench_function("std_once_lock_get_or_init", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let o = Arc::new(std::sync::OnceLock::<u64>::new());

            measure_threads(THREADS, total, || {
                let o = Arc::clone(&o);

                move |n| {
                    for _ in 0..n
                    {
                        black_box(*o.get_or_init(make_42));
                    }
                }
            })
        });
    });

    group.bench_function("resync_once_init", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let o = Arc::new(OsOnce::new());

            measure_threads(THREADS, total, || {
                let o = Arc::clone(&o);

                move |n| {
                    for _ in 0..n
                    {
                        black_box(*o.init(make_42).unwrap());
                    }
                }
            })
        });
    });

    group.bench_function("resync_lazy_force", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let l = Arc::new(OsLazy::new(make_42));

            measure_threads(THREADS, total, || {
                let l = Arc::clone(&l);

                move |n| {
                    for _ in 0..n
                    {
                        black_box(&*l);
                    }
                }
            })
        });
    });
}

criterion_group!(
    once_lazy,
    initialized_access,
    initialized_contention,
    first_touch_contention
);
criterion_main!(once_lazy);
