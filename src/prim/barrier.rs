//! A barrier synchronization primitive that blocks threads until a set number
//! of threads have reached it.

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::{DEFAULT_EPSILON, ISpin, SpinResult};

/// A barrier enables multiple threads to synchronize the beginning of some
/// computation.
///
/// Uses a generation counter to avoid the ABA problem and to correctly
/// handle multiple waves of threads. On Linux with `std`, parking is
/// done via futex on an internal [`AtomicU32`].
#[allow(missing_debug_implementations)]
pub struct Barrier<S: ISpin = crate::spin::DefaultSpin>
{
    count: usize,
    state: AtomicUsize,
    #[cfg(all(feature = "std", target_os = "linux"))]
    futex: AtomicU32,
    spin:  S,
}

impl<S: ISpin> Barrier<S>
{
    /// Creates a new barrier that will block until `count` threads call
    /// [`wait`](Barrier::wait).
    pub fn new(count: usize) -> Option<Self>
    {
        if count == 0
        {
            return None
        }
        Some(Self {
            count,
            state: AtomicUsize::new(0),
            #[cfg(all(feature = "std", target_os = "linux"))]
            futex: AtomicU32::new(0),
            spin: S::default(),
        })
    }

    /// Blocks the current thread until all `count` threads have called this
    /// method on the same barrier.
    ///
    /// # Returns
    /// - `true` if this thread was the **last** to arrive.
    /// - `false` if this thread was among the earlier arrivals.
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
                    #[cfg(all(feature = "std", target_os = "linux"))]
                    {
                        self.futex
                            .store(new_generation as u32, Ordering::Release);
                        futex_wake_all(&self.futex);
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
                        iteration += 1;

                        match self.spin.spin()
                        {
                            SpinResult::Ok =>
                            {
                                #[cfg(all(
                                    feature = "std",
                                    target_os = "linux"
                                ))]
                                if iteration >= DEFAULT_EPSILON
                                {
                                    iteration = 0;
                                    futex_wait(&self.futex, generation as u32);
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
            }
        }
    }

    /// Returns the number of threads that must reach the barrier.
    pub fn count(&self) -> usize
    {
        self.count
    }

    /// Returns the number of threads that have arrived in the current
    /// generation. This is a snapshot and may be immediately outdated.
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

#[cfg(all(feature = "std", target_os = "linux"))]
const FUTEX_WAIT: i32 = 0;
#[cfg(all(feature = "std", target_os = "linux"))]
const FUTEX_WAKE: i32 = 1;
#[cfg(all(feature = "std", target_os = "linux"))]
const FUTEX_PRIVATE_FLAG: i32 = 128;

#[cfg(all(feature = "std", target_os = "linux"))]
#[inline]
fn futex_wait(atomic: &AtomicU32, expected: u32)
{
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            atomic.as_ptr(),
            FUTEX_WAIT | FUTEX_PRIVATE_FLAG,
            expected,
            core::ptr::null::<libc::timespec>(),
        );
    }
}

#[cfg(all(feature = "std", target_os = "linux"))]
#[inline]
fn futex_wake_all(atomic: &AtomicU32)
{
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            atomic.as_ptr(),
            FUTEX_WAKE | FUTEX_PRIVATE_FLAG,
            i32::MAX,
        );
    }
}
