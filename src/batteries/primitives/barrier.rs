//! A synchronization primitive that blocks a set of threads until all of them
//! have reached a certain point.
//!
//! This module provides the [`Barrier`] struct, which allows multiple threads
//! to synchronize their execution phases. Unlike standard library barriers that
//! hardcode their waiting strategies, `resync::Barrier` is generic over the
//! [`RetryPolicy`](crate::traits::RetryPolicy) used to wait when the barrier
//! is not yet full.
//!
//! # Examples
//!
//! ```rust
//! # use resync::Barrier;
//! # use std::sync::Arc;
//! # use std::thread;
//! let barrier = Arc::new(Barrier::<resync::retry::Yield>::new(3));
//! let mut handles = vec![];
//!
//! for i in 0..3
//! {
//!     let b = Arc::clone(&barrier);
//!     handles.push(thread::spawn(move || {
//!         println!("Thread {i} working...");
//!         let result = b.wait().unwrap();
//!         if result.is_leader()
//!         {
//!             println!("Thread {i} is the leader!");
//!         }
//!         println!("Thread {i} passed the barrier.");
//!     }));
//! }
//!
//! for handle in handles
//! {
//!     handle.join().unwrap();
//! }
//! ```

use crate::traits::RetryPolicy;
use core::sync::atomic::{AtomicUsize, Ordering};

/// The result of a [`Barrier::wait`] operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarrierWaitResult
{
    /// Indicates whether the current thread is the "leader" (the last thread
    /// to arrive at the barrier).
    ///
    /// Exactly one thread will receive `true` for each barrier cycle, which
    /// can be useful for performing cleanup or setup tasks before the next
    /// phase of execution.
    is_leader: bool,
}

impl BarrierWaitResult
{
    /// Returns `true` if the current thread is the "leader" (the last thread
    /// to arrive at the barrier).
    pub fn is_leader(&self) -> bool
    {
        self.is_leader
    }
}

/// A barrier that blocks a set of threads until all of them have called
/// [`wait`](Barrier::wait).
///
/// The barrier is initialized with a specific number of threads `n`. Each
/// thread calls `wait()`, which blocks until `n` threads have called it.
/// Once the `n`-th thread arrives, all threads are released, and the barrier
/// resets for the next cycle (allowing it to be reused).
///
/// By default, it uses [`crate::retry::Yield`] as the retry policy (when the
/// `std` feature is enabled).
#[allow(missing_debug_implementations)]
pub struct Barrier<R = crate::retry::Yield>
where R: RetryPolicy
{
    count:      AtomicUsize,
    generation: AtomicUsize,
    n:          usize,
    retry:      R,
}

impl<R> core::fmt::Debug for Barrier<R>
where R: RetryPolicy
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.debug_struct("Barrier")
            .field("n", &self.n)
            .field("count", &self.count.load(Ordering::Relaxed))
            .field("generation", &self.generation.load(Ordering::Relaxed))
            .finish()
    }
}

// SAFETY:
// The barrier uses atomic operations for all state transitions, making it safe
// to share across threads.
unsafe impl<R> Sync for Barrier<R> where R: RetryPolicy + Sync {}
unsafe impl<R> Send for Barrier<R> where R: RetryPolicy + Send {}

impl<R> Barrier<R>
where R: RetryPolicy + Default
{
    /// Creates a new barrier that will block until `n` threads have called
    /// [`wait`](Barrier::wait).
    ///
    /// The retry policy is initialized using its `Default` implementation.
    ///
    /// # Panics
    ///
    /// Panics if `n` is 0.
    pub fn new(n: usize) -> Self
    {
        assert!(n > 0, "Barrier must be initialized with at least 1 thread");
        Self {
            count: AtomicUsize::new(0),
            generation: AtomicUsize::new(0),
            n,
            retry: R::default(),
        }
    }
}

impl<R> Barrier<R>
where R: RetryPolicy
{
    /// Creates a new barrier with a custom retry policy.
    ///
    /// # Panics
    ///
    /// Panics if `n` is 0.
    pub fn with_retry(n: usize, retry: R) -> Self
    {
        assert!(n > 0, "Barrier must be initialized with at least 1 thread");
        Self {
            count: AtomicUsize::new(0),
            generation: AtomicUsize::new(0),
            n,
            retry,
        }
    }

    /// Blocks the current thread until all `n` threads have called this method.
    ///
    /// # Returns
    ///
    /// - `Ok(BarrierWaitResult)`: The barrier was successfully passed. The
    ///   `is_leader` field indicates if this thread was the last to arrive.
    /// - `Err(R::Error)`: The retry policy aborted the wait loop (e.g., due to
    ///   a timeout).
    pub fn wait(&self) -> Result<BarrierWaitResult, <R as RetryPolicy>::Error>
    {
        let gen_ = self.generation.load(Ordering::Acquire);
        let prev = self.count.fetch_add(1, Ordering::AcqRel);

        if prev + 1 == self.n
        {
            // Last thread to arrive: reset count and advance generation
            self.count.store(0, Ordering::Release);
            self.generation.fetch_add(1, Ordering::Release);
            return Ok(BarrierWaitResult { is_leader: true });
        }

        // Wait for the generation to advance
        let mut iterations = 0;
        loop
        {
            iterations += 1;
            if self.generation.load(Ordering::Acquire) != gen_
            {
                return Ok(BarrierWaitResult { is_leader: false });
            }
            self.retry.retry(iterations)?;
        }
    }
}
