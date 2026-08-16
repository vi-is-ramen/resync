#![allow(type_alias_bounds)]

//! A condition variable primitive.
//!
//! This module provides the [`Condvar`] struct, which allows threads to wait
//! for a condition to become true. It is designed to work with `resync`'s
//! [`Mutex`] and [`ExGuard`].
//!
//! # Design
//!
//! Unlike OS-level condition variables that require kernel support, this
//! implementation uses `std::thread::park` and `Thread::unpark` to manage
//! waiting threads. The internal wait queue is protected by a fast,
//! non-blocking atomic spinlock (`lock::Atomic` + `retry::Busy`), ensuring
//! minimal overhead when registering or waking threads.
//!
//! # Poisoning
//!
//! Because `Condvar` releases and reacquires the user-provided [`Mutex`], it
//! fully respects the lock's poisoning semantics. If another thread panics
//! while holding the mutex, the subsequent reacquisition in
//! [`wait`](Self::wait) or [`wait_timeout`](Self::wait_timeout) will return an
//! [`AcquireError::Poisoned`] error, allowing the caller to handle the
//! inconsistent state.

use crate::traits::{LockPolicy, RetryPolicy};
use crate::{AcquireError, ExGuard, Mutex};
use std::collections::VecDeque;
use std::thread::{self, Thread};

/// A condition variable that allows threads to wait for a specific condition
/// to become true.
///
/// This primitive must be used in conjunction with a [`Mutex`]. When a thread
/// calls [`wait`](Self::wait), it atomically releases the associated lock and
/// goes to sleep until another thread calls [`notify_one`](Self::notify_one)
/// or [`notify_all`](Self::notify_all).
#[cfg(feature = "std")]
pub struct Condvar
{
    waiters: Mutex<VecDeque<Thread>, crate::lock::Atomic, crate::retry::Busy>,
}

/// TODO: documentation
pub type CondvarWaitTimeoutResult<'a, T, L, R, M>
where
    L: LockPolicy<Meta = M> + Default,
    R: RetryPolicy + Default,
= Result<
    (ExGuard<'a, T, L, M>, WaitTimeoutResult),
    AcquireError<ExGuard<'a, T, L, M>, L::Error, R::Error>,
>;

/// TODO: documentation
pub type CondvarWaitResult<'a, T, L, R, M>
where
    L: LockPolicy<Meta = M> + Default,
    R: RetryPolicy + Default,
= Result<
    ExGuard<'a, T, L, M>,
    AcquireError<
        ExGuard<'a, T, L, M>,
        <L as LockPolicy>::Error,
        <R as RetryPolicy>::Error,
    >,
>;

#[cfg(feature = "std")]
impl Condvar
{
    /// Creates a new condition variable.
    pub fn new() -> Self
    {
        Self {
            waiters: Mutex::new(VecDeque::new()),
        }
    }

    /// Blocks the current thread until the condition variable is notified.
    ///
    /// This method consumes the current [`ExGuard`], atomically releases the
    /// associated lock, and puts the current thread to sleep. When the thread
    /// is woken up, it will reacquire the lock before returning a new
    /// [`ExGuard`].
    ///
    /// # Spurious Wakeups
    ///
    /// Threads may wake up spuriously even if not explicitly notified. It is
    /// highly recommended to always use `wait` inside a `while` loop that
    /// checks the underlying condition.
    ///
    /// # Errors
    ///
    /// Returns an [`AcquireError`] if the underlying mutex was poisoned by a
    /// panicking thread while this thread was sleeping, or if a fatal
    /// lock/retry error occurs during reacquisition.
    pub fn wait<'a, T, L, R, M>(
        &self,
        guard: ExGuard<'a, T, L, M>,
        mutex: &'a Mutex<T, L, R>,
    ) -> CondvarWaitResult<'a, T, L, R, M>
    where
        L: LockPolicy<Meta = M> + Default,
        R: RetryPolicy + Default,
    {
        // 1. Register the current thread in the wait queue.
        {
            let mut q = self.waiters.lock().unwrap();
            q.push_back(thread::current());
        }

        // 2. Release the user-provided lock.
        drop(guard);

        // 3. Park the thread.
        // If `notify_one` or `notify_all` was called between dropping the
        // guard and parking, the unpark token is already set, and `park()`
        // will return immediately without blocking.
        thread::park();

        // 4. Reacquire the user-provided lock.
        // This correctly propagates `AcquireError::Poisoned` if the mutex
        // was poisoned while we were sleeping.
        mutex.lock()
    }

    /// Blocks the current thread until the condition variable is notified
    /// or the specified timeout has elapsed.
    ///
    /// Returns a tuple containing the reacquired [`ExGuard`] and a
    /// [`WaitTimeoutResult`] indicating whether the wait timed out.
    ///
    /// # Errors
    ///
    /// Returns an [`AcquireError`] if the underlying mutex was poisoned by a
    /// panicking thread, or if a fatal lock/retry error occurs during
    /// reacquisition.
    pub fn wait_timeout<'a, T, L, R, M>(
        &self,
        guard: ExGuard<'a, T, L, M>,
        mutex: &'a Mutex<T, L, R>,
        dur: std::time::Duration,
    ) -> CondvarWaitTimeoutResult<'a, T, L, R, M>
    where
        L: LockPolicy<Meta = M> + Default,
        R: RetryPolicy + Default,
    {
        let current = thread::current();
        {
            let mut q = self.waiters.lock().unwrap();
            q.push_back(current.clone());
        }

        drop(guard);
        thread::park_timeout(dur);

        // Check if we were explicitly notified or if it was a timeout/spurious
        // wakeup.
        let timed_out = {
            let mut q = self.waiters.lock().unwrap();
            if let Some(pos) = q.iter().position(|t| t.id() == current.id())
            {
                q.remove(pos);
                true
            }
            else
            {
                false
            }
        };

        let guard = mutex.lock()?;
        Ok((guard, WaitTimeoutResult(timed_out)))
    }

    /// Wakes up one thread waiting on this condition variable.
    ///
    /// If multiple threads are waiting, it is unspecified which one is woken.
    pub fn notify_one(&self)
    {
        let mut q = self.waiters.lock().unwrap();
        if let Some(t) = q.pop_front()
        {
            drop(q);
            t.unpark();
        }
    }

    /// Wakes up all threads waiting on this condition variable.
    pub fn notify_all(&self)
    {
        let mut waiters = {
            let mut q = self.waiters.lock().unwrap();
            q.drain(..).collect::<Vec<_>>()
        };
        for t in waiters.drain(..)
        {
            t.unpark();
        }
    }
}

#[cfg(feature = "std")]
impl Default for Condvar
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[cfg(feature = "std")]
impl core::fmt::Debug for Condvar
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.write_str("Condvar { ... }")
    }
}

/// Result of a [`Condvar::wait_timeout`] operation.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitTimeoutResult(bool);

#[cfg(feature = "std")]
impl WaitTimeoutResult
{
    /// Returns `true` if the wait timed out (or was spuriously woken up
    /// without being explicitly notified).
    pub fn timed_out(&self) -> bool
    {
        self.0
    }
}
