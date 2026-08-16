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
//! non-blocking atomic spinlock (`lock::Atomic` + `retry::Busy` +
//! `poison::NoPoison`), ensuring minimal overhead when registering or waking
//! threads.
//!
//! # Poisoning
//!
//! Because `Condvar` releases and reacquires the user-provided [`Mutex`], it
//! fully respects the lock's poisoning semantics (defined by its
//! `PoisonPolicy`). If another thread panics while holding the mutex, the
//! subsequent reacquisition in [`wait`](Self::wait) or
//! [`wait_timeout`](Self::wait_timeout) will return an
//! [`AcquireError::Poisoned`] error, allowing the caller to handle the
//! inconsistent state.

use crate::traits::{LockPolicy, PoisonPolicy, RetryPolicy};
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
    waiters: Mutex<
        VecDeque<Thread>,
        crate::lock::Atomic,
        crate::retry::Busy,
        crate::poison::NoPoison,
    >,
}

/// Result type for [`Condvar::wait_timeout`] operations.
pub type CondvarWaitTimeoutResult<'a, T, L, R, P, M>
where
    L: LockPolicy<Meta = M> + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy,
= Result<
    (ExGuard<'a, T, L, P, M>, WaitTimeoutResult),
    AcquireError<ExGuard<'a, T, L, P, M>, L::Error, R::Error>,
>;

/// Result type for [`Condvar::wait`] operations.
pub type CondvarWaitResult<'a, T, L, R, P, M>
where
    L: LockPolicy<Meta = M> + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy,
= Result<
    ExGuard<'a, T, L, P, M>,
    AcquireError<
        ExGuard<'a, T, L, P, M>,
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
    pub fn wait<'a, T, L, R, P, M>(
        &self,
        guard: ExGuard<'a, T, L, P, M>,
        mutex: &'a Mutex<T, L, R, P>,
    ) -> CondvarWaitResult<'a, T, L, R, P, M>
    where
        L: LockPolicy<Meta = M> + Default,
        R: RetryPolicy + Default,
        P: PoisonPolicy,
    {
        {
            let mut q = self.waiters.lock().unwrap();
            q.push_back(thread::current());
        }

        drop(guard);
        thread::park();

        mutex.lock()
    }

    /// Blocks the current thread until the condition variable is notified
    /// or the specified timeout has elapsed.
    pub fn wait_timeout<'a, T, L, R, P, M>(
        &self,
        guard: ExGuard<'a, T, L, P, M>,
        mutex: &'a Mutex<T, L, R, P>,
        dur: std::time::Duration,
    ) -> CondvarWaitTimeoutResult<'a, T, L, R, P, M>
    where
        L: LockPolicy<Meta = M> + Default,
        R: RetryPolicy + Default,
        P: PoisonPolicy,
    {
        let current = thread::current();
        {
            let mut q = self.waiters.lock().unwrap();
            q.push_back(current.clone());
        }

        drop(guard);
        thread::park_timeout(dur);

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
