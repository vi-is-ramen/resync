//! A sharing-yielding (shield) lock wrapper.
//!
//! This module provides the [`Shield`] wrapper, which implements both
//! [`LockPolicy`] and [`SharingPolicy`]. It is designed to prevent writer
//! starvation in read-write lock scenarios by "shielding" the resource from
//! new readers when a writer is waiting to acquire the lock.
//!
//! When a writer attempts to acquire the lock and fails due to contention,
//! it increments a pending writer counter. Subsequent reader acquisition
//! attempts will check this counter; if it is greater than zero, the reader
//! receives [`crate::LockStatus::Fail`], forcing it to yield via its retry
//! policy. This gives the waiting writer a fair chance to acquire the lock
//! once the current readers release it.
use crate::traits::{LockPolicy, NewLocked, SharingPolicy};
use core::sync::atomic::{AtomicUsize, Ordering};

/// A sharing-yielding (shield) lock wrapper.
///
/// This wrapper implements both [`LockPolicy`] and [`SharingPolicy`].
/// It is designed to prevent writer starvation in read-write lock scenarios
/// by "shielding" the resource from new readers when a writer is waiting
/// (or attempting to acquire the lock).
///
/// # Behavior
///
/// - **Exclusive Access (`LockPolicy`)**: When a writer attempts to acquire the
///   lock via [`LockPolicy::try_lock`], it delegates to the inner lock. If the
///   acquisition fails due to contention ([`crate::LockStatus::Fail`]), it
///   increments a pending writer counter. Once the writer successfully acquires
///   the lock ([`crate::LockStatus::Done`]), the counter is decremented.
/// - **Shared Access (`SharingPolicy`)**: When a reader attempts to acquire the
///   lock via [`SharingPolicy::try_share`], it first checks the pending writer
///   counter. If there are any pending writers, the reader acquisition returns
///   [`crate::LockStatus::Fail`]. This forces the reader's retry policy to spin
///   or yield, preventing writer starvation.
///
/// # Type Parameters
///
/// - `L`: The underlying lock policy that must implement [`SharingPolicy`] (and
///   by extension, [`LockPolicy`]).
#[derive(Debug, Default)]
pub struct Shield<L>
where L: LockPolicy
{
    inner:   L,
    pending: AtomicUsize,
}

unsafe impl<L> core::marker::Sync for Shield<L> where L: LockPolicy {}
unsafe impl<L> core::marker::Send for Shield<L> where L: LockPolicy + core::marker::Send
{}

/// Errors that can occur when using the [`Shield`] lock wrapper.
#[derive(Debug)]
pub enum ShieldError<E>
where E: core::error::Error
{
    /// A writer is currently waiting for (or attempting to acquire) the
    /// resource.
    ///
    /// *Note: In the current implementation, `try_share` returns
    /// [`crate::LockStatus::Fail`] to trigger the retry policy instead of
    /// returning this error. This variant is reserved for future strict-mode
    /// implementations where readers might need to abort instead of retry.*
    Writer,
    /// An underlying error occurred in the wrapped lock policy.
    Lock(E),
}

impl<E> core::fmt::Display for ShieldError<E>
where E: core::error::Error
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        match self
        {
            Self::Writer => f.write_str("Writer is waiting for this resource"),
            Self::Lock(e) => <E as core::fmt::Display>::fmt(e, f),
        }
    }
}

impl<E> core::error::Error for ShieldError<E>
where E: core::error::Error
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)>
    {
        match self
        {
            Self::Writer => None,
            Self::Lock(e) => e.source(),
        }
    }
}

unsafe impl<L> LockPolicy for Shield<L>
where L: LockPolicy
{
    type Error = ShieldError<L::Error>;
    type Meta = <L as LockPolicy>::Meta;

    /// Attempts to acquire the lock for exclusive (writer) access.
    ///
    /// If the inner lock acquisition fails due to contention
    /// ([`crate::LockStatus::Fail`]), this method increments the pending
    /// writer counter to signal waiting readers to yield. If the acquisition
    /// succeeds ([`crate::LockStatus::Done`]), it decrements the counter.
    ///
    /// # Safety
    ///
    /// See [`LockPolicy::try_lock`].
    unsafe fn try_lock(
        &self,
        current_iteration: usize,
    ) -> crate::LockResult<Self::Meta, Self::Error>
    {
        match unsafe { self.inner.try_lock(current_iteration) }
        {
            Ok(crate::LockStatus::Done(meta)) =>
            {
                // The writer successfully acquired the lock.
                // Decrement the pending writer count if it's greater than zero.
                let _ = self.pending.try_update(
                    Ordering::Release,
                    Ordering::Relaxed,
                    |x| if x > 0 { Some(x - 1) } else { None },
                );
                Ok(crate::LockStatus::Done(meta))
            },
            Ok(crate::LockStatus::Fail) =>
            {
                // Contention: the writer must wait. Increment the pending
                // writer count to signal readers to yield.
                if current_iteration == 1
                {
                    self.pending.fetch_add(1, Ordering::Release);
                }

                Ok(crate::LockStatus::Fail)
            },
            Err(x) =>
            {
                // Unrecoverable error in the underlying lock.
                Err(ShieldError::Lock(x))
            },
        }
    }

    /// Releases the exclusive (writer) lock.
    ///
    /// # Safety
    ///
    /// See [`LockPolicy::free`].
    unsafe fn free(&self, meta: &Self::Meta)
    {
        unsafe { self.inner.free(meta) }
    }

    /// Wakes all threads waiting on this lock.
    fn wake_all(&self)
    {
        self.inner.wake_all()
    }

    fn abort(&self)
    {
        self.pending.fetch_sub(1, Ordering::Acquire);
    }
}

impl<L> NewLocked for Shield<L>
where L: NewLocked + LockPolicy
{
    /// Creates a new [`Shield`] wrapper with the inner lock already acquired.
    fn new_locked() -> (Self::Meta, Self)
    {
        let (meta, inner) = L::new_locked();
        (
            meta,
            Self {
                inner,
                pending: AtomicUsize::new(0),
            },
        )
    }
}

unsafe impl<L> SharingPolicy for Shield<L>
where L: SharingPolicy
{
    /// Attempts to acquire the lock for shared (reader) access.
    ///
    /// If there are any pending writers (indicated by the pending counter
    /// being non-zero), this method returns [`crate::LockStatus::Fail`] to
    /// force the reader to yield and retry later, preventing writer starvation.
    /// Otherwise, it delegates to the inner lock's
    /// [`SharingPolicy::try_share`].
    fn try_share(
        &self,
        current_iteration: usize,
    ) -> crate::LockResult<Self::Meta, Self::Error>
    {
        if self.pending.load(Ordering::Acquire) != 0
        {
            // Return Fail (not an Error) so the caller's retry policy
            // will spin/yield instead of aborting the acquisition.
            Ok(crate::LockStatus::Fail)
        }
        else
        {
            match self.inner.try_share(current_iteration)
            {
                Ok(x) => Ok(x),
                Err(x) => Err(ShieldError::Lock(x)),
            }
        }
    }

    /// Releases a shared (reader) lock.
    fn free_share(&self, meta: &Self::Meta)
    {
        self.inner.free_share(meta);
    }

    /// Wakes all threads waiting for a shared lock.
    fn wake_readers(&self)
    {
        self.inner.wake_readers();
    }
}
