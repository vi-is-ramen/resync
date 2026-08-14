//! A barrier synchronization primitive that blocks threads until a set number
//! of threads have reached it.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{DEFAULT_EPSILON, ISpin, SpinResult};

/// A barrier enables multiple threads to synchronize the beginning of some
/// computation.
///
/// A barrier is created with a count of `n`. When `n` threads call [`wait`],
/// they all block until the last thread arrives, at which point all threads
/// are released simultaneously. The barrier is then automatically reset and
/// can be reused for the next generation.
///
/// This implementation uses a two-phase state with a generation counter to
/// avoid the ABA problem and to correctly handle multiple waves of threads.
///
/// # Platform Support
///
/// - **`std` feature enabled**: Uses `std::sync::Condvar` for efficient
///   kernel-level parking after the spin phase. This is cross-platform and
///   automatically uses the best mechanism for each OS (futex on Linux,
///   ConditionVariable on Windows, pthread_cond on macOS).
/// - **`std` feature disabled**: Falls back to spin-only waiting via the
///   [`ISpin`] strategy. Suitable for `no_std` environments.
///
/// # Examples
///
/// ```ignore
/// use resync::spin::DefaultSpin;
/// use resync::Barrier;
/// use std::thread;
///
/// let barrier = Barrier::<DefaultSpin>::new(3).unwrap();
/// let mut handles = vec![];
///
/// for _ in 0..3
/// {
///     let b = &barrier;
///     handles.push(thread::spawn(move || {
///         b.wait();
///     }));
/// }
/// for handle in handles
/// {
///     handle.join().unwrap();
/// }
/// ```
///
/// # Type Parameters
/// - `S`: the spin strategy used while waiting (must implement [`ISpin`]).
///   Defaults to [`crate::spin::DefaultSpin`].
///
/// # Limitations
/// - Fairness is not guaranteed; threads may be released in any order.
///
/// [`wait`]: Barrier::wait
#[allow(missing_debug_implementations)]
pub struct Barrier<S: ISpin = crate::spin::DefaultSpin>
{
    count:   usize,
    state:   AtomicUsize,
    spin:    S,
    #[cfg(feature = "std")]
    inner:   std::sync::Mutex<()>,
    #[cfg(feature = "std")]
    condvar: std::sync::Condvar,
}

impl<S: ISpin> Barrier<S>
{
    /// Creates a new barrier that will block until `count` threads call
    /// [`wait`].
    pub fn new(count: usize) -> Option<Self>
    {
        if count == 0
        {
            return None
        }

        Some(Self {
            count,
            state: AtomicUsize::new(0),
            spin: S::default(),
            #[cfg(feature = "std")]
            inner: std::sync::Mutex::new(()),
            #[cfg(feature = "std")]
            condvar: std::sync::Condvar::new(),
        })
    }

    /// Blocks the current thread until all `count` threads have called this
    /// method on the same barrier.
    ///
    /// The call returns immediately if the current thread is the last to
    /// arrive. Otherwise, it waits until the barrier releases the current
    /// generation.
    ///
    /// # Returns
    /// - `true` if this thread was the **last** to arrive (i.e., it triggered
    ///   the release).
    /// - `false` if this thread was among the earlier arrivals.
    ///
    /// # Panics
    /// This method panics if the spin implementation returns `Abort`.
    pub fn wait(&self) -> bool
    {
        loop
        {
            let state = self.state.load(Ordering::Acquire);
            let generation = state >> 32;
            let arrived = state & 0xFFFFFFFF;

            if arrived + 1 == self.count
            {
                // Last arrival: advance the generation and reset the counter.
                let new_generation = generation.wrapping_add(1);
                let new_state = new_generation << 32;
                if self
                    .state
                    .compare_exchange(
                        state,
                        new_state,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    // Wake all waiters
                    #[cfg(feature = "std")]
                    {
                        self.condvar.notify_all();
                    }
                    return true;
                }
                continue;
            }
            else
            {
                // Not the last: try to increment the arrived count.
                let new_state = state + 1;
                if self
                    .state
                    .compare_exchange(
                        state,
                        new_state,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    let mut iteration = 0usize;

                    // Spin until the generation changes
                    while (self.state.load(Ordering::Acquire) >> 32)
                        == generation
                    {
                        match self.spin.spin()
                        {
                            SpinResult::Ok =>
                            {
                                #[cfg(feature = "std")]
                                {
                                    iteration += 1;
                                    if iteration >= DEFAULT_EPSILON
                                    {
                                        // Park via Condvar
                                        let guard = self.inner.lock().unwrap();
                                        let current_gen =
                                            self.state.load(Ordering::Acquire)
                                                >> 32;

                                        // Check again under the lock to
                                        // avoid lost wakeups
                                        if current_gen == generation
                                        {
                                            // intended
                                            #[allow(let_underscore_lock)]
                                            let _ = self
                                                .condvar
                                                .wait(guard)
                                                .unwrap();
                                        }
                                        iteration = 0;
                                    }
                                }
                                #[cfg(not(feature = "std"))]
                                {
                                    // Pure spin in no_std mode
                                    let _ = iteration;
                                }
                                continue
                            },
                            SpinResult::Abort =>
                            {
                                panic!("spin abort during barrier wait");
                            },
                        }
                    }
                    return false;
                }
                // CAS failed; retry from the top.
            }
        }
    }

    /// Returns the number of threads that must reach the barrier to release it.
    pub fn count(&self) -> usize
    {
        self.count
    }

    /// Returns the number of threads that have already arrived at the barrier
    /// in the current generation.
    ///
    /// This is a snapshot and may become outdated immediately.
    pub fn arrived(&self) -> usize
    {
        self.state.load(Ordering::Relaxed) & 0xFFFFFFFF
    }
}

impl<S: ISpin> core::default::Default for Barrier<S>
{
    /// Creates a barrier with a count of 1 (i.e., does not block).
    fn default() -> Self
    {
        Self::new(1).unwrap()
    }
}

#[cfg(all(test, feature = "std"))]
mod tests
{
    use super::*;
    use crate::spin::Busy;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn barrier_basic()
    {
        let barrier = Barrier::<Busy>::new(1).unwrap();
        assert!(barrier.wait());
    }

    #[test]
    fn barrier_multiple_threads()
    {
        let barrier = Arc::new(Barrier::<Busy>::new(3).unwrap());
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut handles = vec![];
        for _ in 0..3
        {
            let b = Arc::clone(&barrier);
            let c = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                c.fetch_add(1, Ordering::Relaxed);
                let is_last = b.wait();
                if is_last
                {
                    c.fetch_add(100, Ordering::Relaxed);
                }
            }));
        }

        for h in handles
        {
            h.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::Relaxed), 103);
    }

    #[test]
    fn barrier_reusable()
    {
        let barrier = Arc::new(Barrier::<Busy>::new(2).unwrap());

        for _ in 0..3
        {
            let b = Arc::clone(&barrier);
            let h1 = thread::spawn(move || {
                b.wait();
            });

            let b = Arc::clone(&barrier);
            let h2 = thread::spawn(move || {
                b.wait();
            });

            h1.join().unwrap();
            h2.join().unwrap();
        }
    }

    #[test]
    fn barrier_count()
    {
        let barrier = Barrier::<Busy>::new(5).unwrap();
        assert_eq!(barrier.count(), 5);
    }

    #[test]
    fn barrier_arrived()
    {
        let barrier = Barrier::<Busy>::new(3).unwrap();
        assert_eq!(barrier.arrived(), 0);
    }

    #[test]
    fn barrier_zero_count_returns_none()
    {
        assert!(Barrier::<Busy>::new(0).is_none());
    }

    #[test]
    fn barrier_default_is_count_one()
    {
        let barrier = Barrier::<Busy>::default();
        assert_eq!(barrier.count(), 1);
        assert!(barrier.wait());
    }

    #[test]
    fn barrier_many_threads()
    {
        let barrier = Arc::new(Barrier::<Busy>::new(10).unwrap());
        let mut handles = vec![];

        for _ in 0..10
        {
            let b = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                b.wait();
            }));
        }

        for h in handles
        {
            h.join().unwrap();
        }
    }

    #[test]
    fn barrier_stress()
    {
        const THREADS: usize = 8;
        const ROUNDS: usize = 100;

        let barrier = Arc::new(Barrier::<Busy>::new(THREADS).unwrap());
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..THREADS
        {
            let b = Arc::clone(&barrier);
            let c = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                for _ in 0..ROUNDS
                {
                    c.fetch_add(1, Ordering::Relaxed);
                    b.wait();
                    c.fetch_add(1, Ordering::Relaxed);
                    b.wait();
                }
            }));
        }

        for h in handles
        {
            h.join().unwrap();
        }

        // Each thread did 2 * ROUNDS increments
        assert_eq!(counter.load(Ordering::Relaxed), THREADS * ROUNDS * 2);
    }
}
