//! A barrier synchronization primitive that blocks threads until a set number
//! of threads have reached it.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{ISpin, SpinResult};

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
/// # Examples
///
/// ```ignore
/// # use resync::spin::Os;
/// use resync::Barrier;
/// use std::thread;
///
/// static barrier: Barrier<Os> = Barrier::new(3).unwrap();
/// let mut handles = vec![];
///
/// for _ in 0..3
/// {
///     let b = &barrier;
///     handles.push(thread::spawn(move || {
///         // Do some work before synchronization...
///         b.wait(); // Wait for all threads to reach this point
///         // All threads continue here together
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
    count: usize,
    state: AtomicUsize, // high 32 bits: generation, low 32 bits: arrived count
    spin:  S,
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
    /// This method panics if spin implementation returns abort and may loop
    /// forever if the barrier is never released.
    ///
    /// # Example
    /// ```ignore
    /// # use resync::spin::Os;
    /// # use resync::Barrier;
    /// # use std::thread;
    /// static barrier: Barrier<Os> = Barrier::new(2).unwrap();
    /// let b = &barrier;
    /// let handle = thread::spawn(move || {
    ///     // thread1
    ///     let is_last = b.wait();
    ///     assert!(!is_last); // thread1 is not the last
    /// });
    /// let is_last = barrier.wait();
    /// assert!(is_last); // main thread is the last
    /// handle.join().unwrap();
    /// ```
    pub fn wait(&self) -> bool
    {
        loop
        {
            let state = self.state.load(Ordering::SeqCst);
            let generation = state >> 32;
            let arrived = state & 0xFFFFFFFF;

            if arrived + 1 == self.count
            {
                // Last arrival: advance the generation and reset the counter.
                let new_state = (generation.wrapping_add(1)) << 32;
                if self
                    .state
                    .compare_exchange(
                        state,
                        new_state,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    return true; // this thread was the last
                }
                // CAS failed: another thread beat us to the reset; we retry.
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
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    // Now spin until the generation changes (i.e., barrier
                    // resets).
                    while (self.state.load(Ordering::SeqCst) >> 32)
                        == generation
                    {
                        match self.spin.spin()
                        {
                            SpinResult::Ok => continue,
                            SpinResult::Abort =>
                            {
                                panic!("spin abort during barrier wait");
                            },
                        }
                    }
                    return false; // not the last, but generation changed – we are released
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
    ///
    /// This is mostly useful for generic contexts where a barrier is required,
    /// but no actual synchronization is needed.
    fn default() -> Self
    {
        Self::new(1).unwrap()
    }
}

// TODO: write useful test, not stubs (moreover, failing)
