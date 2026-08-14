#![allow(missing_docs)]

#[macro_use]
extern crate criterion;

use criterion::{Bencher, Criterion};
use std::hint::black_box;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::thread;

// ---------- Trait abstraction ----------
trait RwLock<T>: Send + Sync + 'static
{
    type ReadGuard<'a>: Deref<Target = T>
    where Self: 'a;
    type WriteGuard<'a>: DerefMut<Target = T>
    where Self: 'a;

    fn new(value: T) -> Self;
    fn read(&self) -> Self::ReadGuard<'_>;
    fn write(&self) -> Self::WriteGuard<'_>;
}

// ---------- resync implementation ----------
impl<T: Send + 'static> RwLock<T> for resync::RwLock<T>
{
    type ReadGuard<'a>
        = resync::RwRef<
        'a,
        T,
        resync::share::DefaultShare,
        resync::spin::DefaultSpin,
    >
    where Self: 'a;
    type WriteGuard<'a>
        = resync::RwMut<
        'a,
        T,
        resync::share::DefaultShare,
        resync::spin::DefaultSpin,
    >
    where Self: 'a;

    fn new(value: T) -> Self
    {
        resync::RwLock::new(value)
    }

    fn read(&self) -> Self::ReadGuard<'_>
    {
        self.read().unwrap()
    }

    fn write(&self) -> Self::WriteGuard<'_>
    {
        self.write().unwrap()
    }
}

// ---------- std implementation ----------
impl<T: Send + 'static + std::marker::Sync> RwLock<T> for std::sync::RwLock<T>
{
    type ReadGuard<'a>
        = std::sync::RwLockReadGuard<'a, T>
    where Self: 'a;
    type WriteGuard<'a>
        = std::sync::RwLockWriteGuard<'a, T>
    where Self: 'a;

    fn new(value: T) -> Self
    {
        std::sync::RwLock::new(value)
    }

    fn read(&self) -> Self::ReadGuard<'_>
    {
        self.read().unwrap()
    }

    fn write(&self) -> Self::WriteGuard<'_>
    {
        self.write().unwrap()
    }
}

// ---------- Bench functions ----------
fn gen_create<R: RwLock<u32>>(b: &mut Bencher)
{
    b.iter(|| {
        let n = black_box(42);
        R::new(n)
    });
}

fn gen_read_unlock<R: RwLock<u32>>(b: &mut Bencher)
{
    let lock = R::new(0);
    b.iter(|| {
        let guard = lock.read();
        black_box(*guard);
        drop(guard);
    });
}

fn gen_write_unlock<R: RwLock<u32>>(b: &mut Bencher)
{
    let lock = R::new(0);
    b.iter(|| {
        let mut guard = lock.write();
        *guard = guard.wrapping_add(1);
        drop(guard);
    });
}

fn gen_read_contention<R: RwLock<u32>>(b: &mut Bencher)
{
    let lock = Arc::new(R::new(0));
    const READER_THREADS: usize = 4;
    let readers: Vec<_> = (0..READER_THREADS)
        .map(|_| {
            let lock = lock.clone();
            thread::spawn(move || {
                while Arc::strong_count(&lock) > 1
                {
                    for _ in 0..1000
                    {
                        black_box(*lock.read());
                    }
                }
            })
        })
        .collect();

    b.iter(|| {
        let mut guard = lock.write();
        *guard = guard.wrapping_add(1);
        drop(guard);
    });

    drop(lock);
    for r in readers
    {
        r.join().unwrap();
    }
}

fn gen_write_contention<R: RwLock<u32>>(b: &mut Bencher)
{
    let lock = Arc::new(R::new(0));
    const WRITER_THREADS: usize = 4;
    let writers: Vec<_> = (0..WRITER_THREADS)
        .map(|_| {
            let lock = lock.clone();
            thread::spawn(move || {
                while Arc::strong_count(&lock) > 1
                {
                    for _ in 0..1000
                    {
                        let mut guard = lock.write();
                        *guard = guard.wrapping_add(1);
                        drop(guard);
                    }
                }
            })
        })
        .collect();

    b.iter(|| {
        let mut guard = lock.write();
        *guard = guard.wrapping_add(1);
        drop(guard);
    });

    drop(lock);
    for w in writers
    {
        w.join().unwrap();
    }
}

fn gen_mixed_contention<R: RwLock<u32>>(b: &mut Bencher)
{
    let lock = Arc::new(R::new(0));
    const THREADS: usize = 4;
    let threads: Vec<_> = (0..THREADS)
        .map(|i| {
            let lock = lock.clone();
            thread::spawn(move || {
                while Arc::strong_count(&lock) > 1
                {
                    for _ in 0..1000
                    {
                        if i % 2 == 0
                        {
                            black_box(*lock.read());
                        }
                        else
                        {
                            let mut guard = lock.write();
                            *guard = guard.wrapping_add(1);
                            drop(guard);
                        }
                    }
                }
            })
        })
        .collect();

    b.iter(|| {
        let mut guard = lock.write();
        *guard = guard.wrapping_add(1);
        drop(guard);
    });

    drop(lock);
    for t in threads
    {
        t.join().unwrap();
    }
}

// ---------- Criterion group ----------
fn criterion_benchmark(c: &mut Criterion)
{
    let mut group = c.benchmark_group("rwlock");

    group.bench_function("create-resync", |b| {
        gen_create::<resync::RwLock<u32>>(b)
    });
    group.bench_function("create-std", |b| {
        gen_create::<std::sync::RwLock<u32>>(b)
    });

    group.bench_function("read_unlock-resync", |b| {
        gen_read_unlock::<resync::RwLock<u32>>(b)
    });
    group.bench_function("read_unlock-std", |b| {
        gen_read_unlock::<std::sync::RwLock<u32>>(b)
    });

    group.bench_function("write_unlock-resync", |b| {
        gen_write_unlock::<resync::RwLock<u32>>(b)
    });
    group.bench_function("write_unlock-std", |b| {
        gen_write_unlock::<std::sync::RwLock<u32>>(b)
    });

    group.bench_function("read_contention-resync", |b| {
        gen_read_contention::<resync::RwLock<u32>>(b)
    });
    group.bench_function("read_contention-std", |b| {
        gen_read_contention::<std::sync::RwLock<u32>>(b)
    });

    group.bench_function("write_contention-resync", |b| {
        gen_write_contention::<resync::RwLock<u32>>(b)
    });
    group.bench_function("write_contention-std", |b| {
        gen_write_contention::<std::sync::RwLock<u32>>(b)
    });

    group.bench_function("mixed_contention-resync", |b| {
        gen_mixed_contention::<resync::RwLock<u32>>(b)
    });
    group.bench_function("mixed_contention-std", |b| {
        gen_mixed_contention::<std::sync::RwLock<u32>>(b)
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
