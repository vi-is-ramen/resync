//! A barrier synchronization primitive.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{DEFAULT_EPSILON, ISpin};

/// A barrier enables multiple threads to synchronize.
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
    /// Creates a new barrier.
    pub fn new(count: usize) -> Option<Self>
    {
        if count == 0
        {
            return None;
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

    /// Blocks until all threads have called `wait`.
    ///
    /// # Returns
    /// - `true`: this thread was the last to arrive
    /// - `false`: this thread was an earlier arrival
    ///
    /// # Errors
    /// Panics if spin aborts.
    pub fn wait(&self) -> bool
    {
        loop
        {
            let state = self.state.load(Ordering::Acquire);
            let generation = state >> 32;
            let arrived = state & 0xFFFFFFFF;

            if arrived + 1 == self.count
            {
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

                    while (self.state.load(Ordering::Acquire) >> 32)
                        == generation
                    {
                        if self.spin.spin().is_err()
                        {
                            panic!("spin aborted during barrier wait");
                        }

                        #[cfg(feature = "std")]
                        {
                            iteration += 1;
                            if iteration >= DEFAULT_EPSILON
                            {
                                let guard = self.inner.lock().unwrap();
                                let current_gen =
                                    self.state.load(Ordering::Acquire) >> 32;

                                if current_gen == generation
                                {
                                    #[allow(let_underscore_lock)]
                                    let _ = self.condvar.wait(guard).unwrap();
                                }
                                iteration = 0;
                            }
                        }
                        #[cfg(not(feature = "std"))]
                        {
                            let _ = iteration;
                        }
                    }
                    return false;
                }
            }
        }
    }

    /// Returns the number of threads that must reach the barrier.
    pub fn count(&self) -> usize
    {
        self.count
    }

    /// Returns the number of threads that have arrived.
    pub fn arrived(&self) -> usize
    {
        self.state.load(Ordering::Relaxed) & 0xFFFFFFFF
    }
}

impl<S: ISpin> core::default::Default for Barrier<S>
{
    fn default() -> Self
    {
        Self::new(1).unwrap()
    }
}
