#![allow(clippy::all, missing_docs)]

mod common;

use common::{THREADS, iters_to_usize, measure_threads, measure_two_roles};
use criterion::{Criterion, criterion_group, criterion_main};
use resync::{Barrier, Condvar, Gate, Semaphore};
use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

type OsMutex<T> =
    resync::Mutex<T, common::OsLock, common::OsRetry, common::OsPoison>;

fn barrier(c: &mut Criterion)
{
    let mut group = c.benchmark_group("barrier/wait");
    group.sample_size(20);

    group.bench_function("std", |b| {
        let bar = Arc::new(std::sync::Barrier::new(THREADS));

        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);

            measure_threads(THREADS, total, || {
                let bar = Arc::clone(&bar);

                move |n| {
                    for _ in 0..n
                    {
                        let _ = black_box(bar.wait());
                    }
                }
            })
        });
    });

    group.bench_function("resync_yield", |b| {
        let bar = Arc::new(Barrier::<common::OsRetry>::new(THREADS));

        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);

            measure_threads(THREADS, total, || {
                let bar = Arc::clone(&bar);

                move |n| {
                    for _ in 0..n
                    {
                        let _ = black_box(bar.wait().unwrap());
                    }
                }
            })
        });
    });

    group.bench_function("resync_busy", |b| {
        let bar = Arc::new(Barrier::<common::SpinRetry>::new(THREADS));

        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);

            measure_threads(THREADS, total, || {
                let bar = Arc::clone(&bar);

                move |n| {
                    for _ in 0..n
                    {
                        let _ = black_box(bar.wait().unwrap());
                    }
                }
            })
        });
    });
}

fn condvar(c: &mut Criterion)
{
    let mut group = c.benchmark_group("condvar/ping_pong");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(2));

    group.bench_function("std", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let mutex = Arc::new(std::sync::Mutex::new(false));
            let cvar = Arc::new(std::sync::Condvar::new());

            measure_two_roles(
                total,
                || {
                    let mutex = Arc::clone(&mutex);
                    let cvar = Arc::clone(&cvar);

                    move |n| {
                        for _ in 0..n
                        {
                            let mut flag = mutex.lock().unwrap();

                            while *flag
                            {
                                flag = cvar.wait(flag).unwrap();
                            }

                            *flag = true;
                            black_box(*flag);
                            cvar.notify_all();
                        }
                    }
                },
                || {
                    let mutex = Arc::clone(&mutex);
                    let cvar = Arc::clone(&cvar);

                    move |n| {
                        for _ in 0..n
                        {
                            let mut flag = mutex.lock().unwrap();

                            while !*flag
                            {
                                flag = cvar.wait(flag).unwrap();
                            }

                            *flag = false;
                            black_box(*flag);
                            cvar.notify_all();
                        }
                    }
                },
            )
        });
    });

    group.bench_function("resync_os", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let mutex = Arc::new(OsMutex::<bool>::new(false));
            let cvar = Arc::new(Condvar::new());

            measure_two_roles(
                total,
                || {
                    let mutex = Arc::clone(&mutex);
                    let cvar = Arc::clone(&cvar);

                    move |n| {
                        for _ in 0..n
                        {
                            let mut flag = mutex.lock().unwrap();

                            while *flag
                            {
                                flag = cvar.wait(flag, &*mutex).unwrap();
                            }

                            *flag = true;
                            black_box(*flag);
                            cvar.notify_all();
                        }
                    }
                },
                || {
                    let mutex = Arc::clone(&mutex);
                    let cvar = Arc::clone(&cvar);

                    move |n| {
                        for _ in 0..n
                        {
                            let mut flag = mutex.lock().unwrap();

                            while !*flag
                            {
                                flag = cvar.wait(flag, &*mutex).unwrap();
                            }

                            *flag = false;
                            black_box(*flag);
                            cvar.notify_all();
                        }
                    }
                },
            )
        });
    });
}

fn gate(c: &mut Criterion)
{
    let mut group = c.benchmark_group("gate/reusable_open_close");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(2));

    macro_rules! bench_gate {
        ($name:expr, $gate_ty:ty) => {
            group.bench_function($name, |b| {
                b.iter_custom(|iters| {
                    let total = iters_to_usize(iters);

                    if total == 0
                    {
                        return Duration::ZERO;
                    }

                    let gate = Arc::new(<$gate_ty>::new());

                    let boot = Arc::new(std::sync::Barrier::new(THREADS + 1));
                    let ready = Arc::new(std::sync::Barrier::new(THREADS + 1));
                    let done = Arc::new(std::sync::Barrier::new(THREADS + 1));

                    let mut handles = Vec::with_capacity(THREADS);

                    for _ in 0..THREADS
                    {
                        let gate = Arc::clone(&gate);
                        let boot = Arc::clone(&boot);
                        let ready = Arc::clone(&ready);
                        let done = Arc::clone(&done);

                        handles.push(std::thread::spawn(move || {
                            let _ = boot.wait();

                            for _ in 0..total
                            {
                                let _ = ready.wait();
                                gate.wait().unwrap();
                                black_box(());
                                let _ = done.wait();
                            }
                        }));
                    }

                    let _ = boot.wait();
                    let started = Instant::now();

                    for i in 0..total
                    {
                        let _ = ready.wait();
                        gate.open();
                        let _ = done.wait();

                        if i + 1 < total
                        {
                            gate.close().unwrap();
                        }
                    }

                    let elapsed = started.elapsed();

                    for handle in handles
                    {
                        handle.join().unwrap();
                    }

                    elapsed
                })
            });
        };
    }

    bench_gate!(
        "resync_os",
        Gate<common::OsLock, common::OsRetry>
    );

    bench_gate!(
        "resync_spin",
        Gate<common::SpinLock, common::SpinRetry>
    );
}

fn semaphore(c: &mut Criterion)
{
    let mut group = c.benchmark_group("semaphore/uncontended");

    group.bench_function("resync_os_acquire_release", |b| {
        let s = Semaphore::<common::OsLock, common::OsRetry>::new(1);

        b.iter(|| {
            s.acquire().unwrap();
            black_box(());
            s.release().unwrap();
        });
    });

    group.bench_function("resync_spin_acquire_release", |b| {
        let s = Semaphore::<common::SpinLock, common::SpinRetry>::new(1);

        b.iter(|| {
            s.acquire().unwrap();
            black_box(());
            s.release().unwrap();
        });
    });

    group.bench_function("resync_os_try_acquire_release", |b| {
        let s = Semaphore::<common::OsLock, common::OsRetry>::new(1);

        b.iter(|| {
            s.try_acquire().unwrap();
            black_box(());
            s.release().unwrap();
        });
    });

    group.bench_function("resync_spin_try_acquire_release", |b| {
        let s = Semaphore::<common::SpinLock, common::SpinRetry>::new(1);

        b.iter(|| {
            s.try_acquire().unwrap();
            black_box(());
            s.release().unwrap();
        });
    });

    group.finish();

    let mut group = c.benchmark_group("semaphore/contended");
    group.sample_size(20);

    group.bench_function("resync_os_one_permit", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let s =
                Arc::new(Semaphore::<common::OsLock, common::OsRetry>::new(1));

            measure_threads(THREADS, total, || {
                let s = Arc::clone(&s);

                move |n| {
                    for _ in 0..n
                    {
                        s.acquire().unwrap();
                        black_box(());
                        s.release().unwrap();
                    }
                }
            })
        });
    });

    group.bench_function("resync_spin_one_permit", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let s = Arc::new(
                Semaphore::<common::SpinLock, common::SpinRetry>::new(1),
            );

            measure_threads(THREADS, total, || {
                let s = Arc::clone(&s);

                move |n| {
                    for _ in 0..n
                    {
                        s.acquire().unwrap();
                        black_box(());
                        s.release().unwrap();
                    }
                }
            })
        });
    });

    group.bench_function("resync_os_many_permits", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let s = Arc::new(
                Semaphore::<common::OsLock, common::OsRetry>::new(THREADS),
            );

            measure_threads(THREADS, total, || {
                let s = Arc::clone(&s);

                move |n| {
                    for _ in 0..n
                    {
                        s.acquire().unwrap();
                        black_box(());
                        s.release().unwrap();
                    }
                }
            })
        });
    });

    group.bench_function("resync_spin_many_permits", |b| {
        b.iter_custom(|iters| {
            let total = iters_to_usize(iters);
            let s = Arc::new(
                Semaphore::<common::SpinLock, common::SpinRetry>::new(THREADS),
            );

            measure_threads(THREADS, total, || {
                let s = Arc::clone(&s);

                move |n| {
                    for _ in 0..n
                    {
                        s.acquire().unwrap();
                        black_box(());
                        s.release().unwrap();
                    }
                }
            })
        });
    });
}

criterion_group!(flow, barrier, condvar, gate, semaphore);
criterion_main!(flow);
