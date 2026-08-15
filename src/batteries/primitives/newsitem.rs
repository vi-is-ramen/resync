//! A synchronization primitive that blocks threads until it is signaled.
//!
//! This module provides the [`NewsItem`] struct, which acts as a manual-reset
//! event (sometimes called a "sticky" event or "gateway"). Threads calling
//! [`wait`](NewsItem::wait) will block until another thread calls
//! [`set`](NewsItem::set). Once set, the event remains in the signaled state,
//! allowing any subsequent calls to `wait` to return immediately, until
//! [`reset`](NewsItem::reset) is called.
//!
//! Unlike standard library condition variables that require a mutex and a
//! wait queue, `resync::NewsItem` is built using atomic operations and a
//! composable [`RetryPolicy`](crate::traits::RetryPolicy).
//!
//! # Examples
//!
//! ```rust
//! # use resync::NewsItem;
//! # use std::sync::Arc;
//! # use std::thread;
//! let event = Arc::new(NewsItem::<resync::retry::Yield>::new());
//! let event_clone = Arc::clone(&event);
//!
//! let handle = thread::spawn(move || {
//!     println!("Worker waiting for event...");
//!     event_clone.wait().unwrap();
//!     println!("Worker received event!");
//! });
//!
//! // Do some work...
//! println!("Main thread signaling event...");
//! event.set();
//!
//! handle.join().unwrap();
//! ```
//!
//! # Limitations
//!
//! Because `NewsItem` relies on the [`RetryPolicy`] to wait (e.g., spinning or
//! yielding) rather than a kernel-managed wait queue, calling [`reset`] while
//! threads are actively waiting inside [`wait`] may cause those threads to
//! miss the signal and block indefinitely until the event is set again. This
//! is a fundamental trade-off of using lock-free atomics combined with
//! spin/yield retry policies instead of OS-level condition variables.

use crate::traits::RetryPolicy;
use core::sync::atomic::{AtomicUsize, Ordering};

/// A manual-reset synchronization event.
///
/// The event has two states: **unset** (default) and **set**.
/// - When **unset**, calls to [`wait`](NewsItem::wait) will block (using the
///   configured [`RetryPolicy`]) until the event is set.
/// - When **set**, calls to `wait` return immediately.
///
/// The event can be transitioned back to the **unset** state by calling
/// [`reset`](NewsItem::reset), making it reusable for multiple synchronization
/// phases.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct NewsItem<R = crate::retry::Yield>
where R: RetryPolicy
{
    /// 0 = unset, 1 = set
    state: AtomicUsize,
    retry: R,
}

impl<R> core::fmt::Debug for NewsItem<R>
where R: RetryPolicy
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.debug_struct("NewsItem")
            .field("is_set", &(self.state.load(Ordering::Relaxed) != 0))
            .finish()
    }
}

// SAFETY: The event uses atomic operations for all state transitions,
// making it safe to share and move across threads.
unsafe impl<R> Sync for NewsItem<R> where R: RetryPolicy + Sync {}
unsafe impl<R> Send for NewsItem<R> where R: RetryPolicy + Send {}

impl<R> NewsItem<R>
where R: RetryPolicy + Default
{
    /// Creates a new, unset `NewsItem`.
    ///
    /// The retry policy is initialized using its `Default` implementation.
    pub fn new() -> Self
    {
        Self {
            state: AtomicUsize::new(0),
            retry: R::default(),
        }
    }
}

impl<R> NewsItem<R>
where R: RetryPolicy
{
    /// Creates a new, unset `NewsItem` with a custom retry policy.
    pub fn with_retry(retry: R) -> Self
    {
        Self {
            state: AtomicUsize::new(0),
            retry,
        }
    }

    /// Signals the event, releasing all current and future waiters.
    ///
    /// Once set, the event remains in the signaled state until
    /// [`reset`](NewsItem::reset) is called.
    pub fn set(&self)
    {
        self.state.store(1, Ordering::Release);
    }

    /// Resets the event to the unset state.
    ///
    /// Future calls to [`wait`](NewsItem::wait) will block until
    /// [`set`](NewsItem::set) is called again.
    ///
    /// # Warning
    ///
    /// Calling `reset` while other threads are actively waiting inside `wait`
    /// may cause those threads to miss the signal and block indefinitely
    /// (or until the event is set again). Ensure that all expected waiters
    /// have passed the barrier before resetting, or use a generation-based
    /// primitive like [`Barrier`](crate::Barrier) if you need strict phase
    /// synchronization.
    pub fn reset(&self)
    {
        self.state.store(0, Ordering::Release);
    }

    /// Returns `true` if the event is currently in the signaled (set) state.
    pub fn is_set(&self) -> bool
    {
        self.state.load(Ordering::Acquire) != 0
    }

    /// Blocks the current thread until the event is signaled.
    ///
    /// If the event is already set, this method returns immediately.
    /// Otherwise, it repeatedly checks the state and invokes the
    /// [`RetryPolicy::retry`] method to wait (e.g., by spinning or yielding).
    ///
    /// # Returns
    ///
    /// - `Ok(())`: The event was signaled.
    /// - `Err(R::Error)`: The retry policy aborted the wait loop (e.g., due to
    ///   a timeout).
    pub fn wait(&self) -> Result<(), <R as RetryPolicy>::Error>
    {
        let mut iterations = 0;
        loop
        {
            if self.state.load(Ordering::Acquire) != 0
            {
                return Ok(());
            }
            self.retry.retry(iterations)?;
            iterations += 1;
        }
    }
}
