//! A synchronization primitive for one-time initialization.
//!
//! This module provides the [`Once`] struct, which ensures that a specific
//! initialization routine is executed exactly once, even when called from
//! multiple threads concurrently. It is similar to [`std::sync::OnceLock`],
//! but built using `resync`'s composable [`LockPolicy`], [`RetryPolicy`],
//! and [`PoisonPolicy`] traits.
//!
//! # Poisoning
//!
//! If the initialization closure panics, the `Once` primitive becomes
//! **poisoned** (if the configured [`PoisonPolicy`] supports it, e.g.,
//! [`crate::poison::StdPoison`]). Subsequent calls to [`init`](Self::init)
//! will immediately return an [`AcquireError::Poisoned`] error, preventing
//! threads from hanging or re-executing a faulty initialization.

use crate::traits::{LockPolicy, PoisonPolicy, RetryPolicy};
use crate::{AcquireError, LockStatus, PoisonError};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU8, Ordering};

const EMPTY: u8 = 0;
const INITIALIZING: u8 = 1;
const DONE: u8 = 2;
const POISONED: u8 = 3;

/// A synchronization primitive for one-time initialization.
///
/// This primitive protects a value of type `T` that is initialized lazily
/// via a closure. It uses a [`LockPolicy`] to synchronize the initialization
/// phase and a [`RetryPolicy`] to wait if another thread is currently
/// performing the initialization.
#[allow(missing_debug_implementations)]
pub struct Once<
    T,
    L = crate::lock::DefaultLock,
    R = crate::retry::DefaultRetry,
    P = crate::poison::DefaultPoison,
> where
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    state:  AtomicU8,
    lock:   L,
    retry:  R,
    poison: P,
    data:   UnsafeCell<Option<T>>,
}

// SAFETY:
// The `Once` primitive ensures exclusive access during initialization and
// immutable access afterwards. It is safe to share across threads.
unsafe impl<T, L, R, P> core::marker::Sync for Once<T, L, R, P>
where
    L: LockPolicy + core::marker::Sync,
    R: RetryPolicy + core::marker::Sync,
    P: PoisonPolicy + core::marker::Sync,
{
}

// SAFETY:
// The `Once` primitive can be safely moved between threads.
unsafe impl<T, L, R, P> core::marker::Send for Once<T, L, R, P>
where
    T: Send,
    L: LockPolicy + core::marker::Send,
    R: RetryPolicy + core::marker::Send,
    P: PoisonPolicy + core::marker::Send,
{
}

impl<T, L, R, P> Once<T, L, R, P>
where
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy + Default,
{
    /// Creates a new, uninitialized `Once` primitive.
    pub fn new() -> Self
    {
        Self {
            state:  AtomicU8::new(EMPTY),
            lock:   L::default(),
            retry:  R::default(),
            poison: P::default(),
            data:   UnsafeCell::new(None),
        }
    }
}

impl<T, L, R, P> core::default::Default for Once<T, L, R, P>
where
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy + Default,
{
    fn default() -> Self
    {
        Self::new()
    }
}

struct LockGuard<'a, L: LockPolicy>
{
    lock: &'a L,
    meta: L::Meta,
}

impl<'a, L: LockPolicy> Drop for LockGuard<'a, L>
{
    fn drop(&mut self)
    {
        unsafe { self.lock.free(&self.meta) };
    }
}

struct PoisonGuard<'a, P: PoisonPolicy>
{
    poison:  &'a P,
    state:   &'a AtomicU8,
    success: bool,
}

impl<'a, P: PoisonPolicy> Drop for PoisonGuard<'a, P>
{
    fn drop(&mut self)
    {
        if !self.success
        {
            P::on_drop(self.poison);
            self.state.store(POISONED, Ordering::Release);
        }
    }
}

impl<T, L, R, P> Once<T, L, R, P>
where
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    /// Initializes the `Once` primitive with the given closure.
    ///
    /// If the primitive is already initialized, this method returns a
    /// reference to the existing value immediately (fast-path).
    ///
    /// If the initialization closure panics, the primitive becomes poisoned
    /// (depending on the `PoisonPolicy`), and subsequent calls will return
    /// an [`AcquireError::Poisoned`] error.
    pub fn init(
        &self,
        f: impl FnOnce() -> T,
    ) -> Result<&T, AcquireError<(), L::Error, R::Error>>
    {
        // Fast-path
        match self.state.load(Ordering::Acquire)
        {
            DONE =>
            {
                return Ok(unsafe {
                    (*self.data.get()).as_ref().unwrap_unchecked()
                });
            },
            POISONED =>
            {
                return Err(AcquireError::Poisoned(PoisonError::new(())));
            },
            _ =>
            {},
        }

        // Slow-path
        let mut iterations = 0usize;
        loop
        {
            iterations += 1;
            match unsafe { self.lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    let lock_guard = LockGuard {
                        lock: &self.lock,
                        meta,
                    };
                    let mut poison_guard = PoisonGuard::<P> {
                        poison:  &self.poison,
                        state:   &self.state,
                        success: false,
                    };

                    let state = self.state.load(Ordering::Acquire);
                    match state
                    {
                        DONE =>
                        {
                            poison_guard.success = true;
                            drop(lock_guard);
                            return Ok(unsafe {
                                (*self.data.get()).as_ref().unwrap_unchecked()
                            });
                        },
                        POISONED =>
                        {
                            poison_guard.success = true;
                            drop(lock_guard);
                            return Err(AcquireError::Poisoned(
                                PoisonError::new(()),
                            ));
                        },
                        INITIALIZING =>
                        {
                            // Another thread is currently running the init
                            // closure. Release the
                            // lock and wait for it to finish.
                            poison_guard.success = true;
                            drop(lock_guard);
                            if let Err(e) = self.retry.retry(iterations)
                            {
                                return Err(AcquireError::Retry(e));
                            }
                            continue;
                        },
                        EMPTY =>
                        {
                            // We are the first thread to reach this point.
                            self.state.store(INITIALIZING, Ordering::Relaxed);

                            let value = f();
                            unsafe {
                                *self.data.get() = Some(value);
                            }

                            self.state.store(DONE, Ordering::Release);
                            poison_guard.success = true;
                            drop(lock_guard);

                            return Ok(unsafe {
                                (*self.data.get()).as_ref().unwrap_unchecked()
                            });
                        },
                        _ => unreachable!(),
                    }
                },
                Ok(LockStatus::Fail) =>
                {
                    if let Err(e) = self.retry.retry(iterations)
                    {
                        return Err(AcquireError::Retry(e));
                    }
                },
                Err(e) =>
                {
                    return Err(AcquireError::Lock(e));
                },
            }
        }
    }

    /// Returns a reference to the initialized value, or `None` if it has
    /// not been initialized yet.
    pub fn get(&self) -> Option<&T>
    {
        if self.state.load(Ordering::Acquire) == DONE
        {
            Some(unsafe { (*self.data.get()).as_ref().unwrap_unchecked() })
        }
        else
        {
            None
        }
    }

    /// Returns `true` if the initialization has successfully completed.
    pub fn is_completed(&self) -> bool
    {
        self.state.load(Ordering::Acquire) == DONE
    }

    /// Returns `true` if the initialization closure panicked and the
    /// primitive is poisoned.
    pub fn is_poisoned(&self) -> bool
    {
        self.state.load(Ordering::Acquire) == POISONED
    }
}
