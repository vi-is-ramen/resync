#![allow(missing_docs)]

#[macro_use]
extern crate criterion;

use criterion::{Bencher, Criterion};
use std::sync::Arc;
use std::thread;

trait Barrier: Send + Sync + 'static
{
    fn new(count: usize) -> Self;
    fn wait(&self);
}

impl Barrier for resync::Barrier<resync::spin::DefaultSpin>
{
    fn new(count: usize) -> Self
    {
        resync::Barrier::new(count).unwrap()
    }

    fn wait(&self)
    {
        self.wait();
    }
}

impl Barrier for std::sync::Barrier
{
    fn new(count: usize) -> Self
    {
        std::sync::Barrier::new(count)
    }

    fn wait(&self)
    {
        self.wait();
    }
}

fn gen_barrier_wait<B: Barrier>(b: &mut Bencher)
{
    const THREADS: usize = 4;
    b.iter(|| {
        let barrier = Arc::new(B::new(THREADS + 1));
        let mut handles = vec![];
        for _ in 0..THREADS
        {
            let b = barrier.clone();
            handles.push(thread::spawn(move || {
                b.wait();
            }));
        }
        barrier.wait();
        for h in handles
        {
            h.join().unwrap();
        }
    });
}

fn criterion_benchmark(c: &mut Criterion)
{
    let mut group = c.benchmark_group("barrier");

    group.bench_function("wait-resync", |b| {
        gen_barrier_wait::<resync::Barrier<resync::spin::DefaultSpin>>(b)
    });
    group.bench_function("wait-std", |b| {
        gen_barrier_wait::<std::sync::Barrier>(b)
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
