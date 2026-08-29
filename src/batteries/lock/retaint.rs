//! A reentrant (recursive) lock wrapper.
//!
//! This module provides the [`Retaint`] wrapper, which adds thread-local
//! reentrancy to any [`LockPolicy`] (and optionally [`SharingPolicy`])
//! implementation.
//!
//! # Behavior
//!
//! - The first successful exclusive acquisition records the current thread
//!   identifier, stores the inner lock's metadata, and sets the guard counter
//!   to one.
//! - Subsequent exclusive acquisitions by the same thread succeed immediately
//!   by incrementing the counter, without touching the inner lock.
//! - Each [`LockPolicy::free`] call decrements the counter. When it reaches
//!   zero, the stored metadata is passed to the inner lock's
//!   [`LockPolicy::free`], actually releasing it.
//! - Shared acquisitions are delegated directly to the inner lock and are not
//!   made reentrant, because multiple threads may hold a shared lock
//!   concurrently and cannot be tracked by a single owner field.
//!
//! # Thread Identification
//!
//! Reentrancy requires a stable identifier for the current thread. The
//! [`StableThreadId`] trait abstracts this. Under the `std` feature, the
//! default implementation [`DefaultThreadId`] uses the address of a
//! thread-local variable. In `#![no_std]` environments you must provide
//! your own implementation and pass it as the second type parameter of
//! [`Retaint`].
//!
//! [`StableThreadId`]: crate::api::StableThreadId
//! [`DefaultThreadId`]: crate::thread_id::DefaultThreadId

use crate::api::{LockPolicy, SharingPolicy, StableThreadId};
use crate::thread_id::DefaultThreadId;
use crate::{LockResult, LockStatus};
use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicUsize, Ordering};

/// A reentrant wrapper around another lock policy.
///
/// `Retaint` tracks which thread currently holds the lock and how many guards
/// that thread has created. Recursive exclusive acquisitions by the owning
/// thread are allowed without blocking. The inner lock is only acquired on the
/// first entry and released on the last exit.
///
/// # Type Parameters
///
/// - `L`: The underlying lock policy.
/// - `T`: The thread identifier provider. Defaults to [`DefaultThreadId`],
///   which requires the `std` feature.
#[allow(missing_debug_implementations)]
pub struct Retaint<L, T = DefaultThreadId>
where L: LockPolicy
{
    inner: L,
    /// Identifier of the thread that currently owns the lock, or `0` if the
    /// lock is unowned.
    owner: AtomicUsize,
    /// Number of active guards held by the owning thread.
    count: AtomicUsize,
    /// Metadata returned by the inner lock on first acquisition.
    ///
    /// Access is synchronized by the ownership state: only the owning thread
    /// may read or write this field while `count > 0`, and it is written only
    /// during the transition `0 -> 1` and read during the transition `1 -> 0`.
    meta:  UnsafeCell<Option<L::Meta>>,
    _tid:  PhantomData<fn() -> T>,
}

// SAFETY:
// The `meta` cell is accessed only by the owning thread while `count > 0`.
// Transitions between zero and non-zero are synchronized by the inner lock's
// acquisition and release, which are themselves atomic.
unsafe impl<L, T> Sync for Retaint<L, T>
where
    L: LockPolicy + Sync,
    L::Meta: Send,
{
}

// SAFETY:
// The wrapper can be moved between threads when no lock is held. The stored
// metadata is `Send`.
unsafe impl<L, T> Send for Retaint<L, T>
where
    L: LockPolicy + Send,
    L::Meta: Send,
{
}

impl<L, T> core::fmt::Debug for Retaint<L, T>
where L: LockPolicy
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.write_str("Retaint { ... }")
    }
}

impl<L, T> Default for Retaint<L, T>
where
    L: LockPolicy + Default,
    T: StableThreadId,
{
    fn default() -> Self
    {
        Self {
            inner: L::default(),
            owner: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
            meta:  UnsafeCell::new(None),
            _tid:  PhantomData,
        }
    }
}

unsafe impl<L, T> LockPolicy for Retaint<L, T>
where
    L: LockPolicy,
    L::Meta: Clone + Send,
    T: StableThreadId,
    T::Id: Copy + Into<usize>,
{
    type Error = L::Error;
    type Meta = L::Meta;

    /// Attempts to acquire the lock, allowing reentrancy for the owning
    /// thread.
    ///
    /// # Safety
    ///
    /// See [`LockPolicy::try_lock`].
    unsafe fn try_lock(
        &self,
        current_iteration: usize,
    ) -> LockResult<Self::Meta, Self::Error>
    {
        let id: usize = T::thread_id().into();
        let owner = self.owner.load(Ordering::Acquire);

        if owner == id && self.count.load(Ordering::Acquire) > 0
        {
            // Reentrant acquisition: just bump the counter and return a clone
            // of the stored metadata.
            self.count.fetch_add(1, Ordering::AcqRel);

            // SAFETY: Only the owning thread accesses `meta` while
            // `count > 0`. We are the owner.
            let meta = unsafe { (*self.meta.get()).clone() }
                .expect("reentrant lock lost metadata");
            return Ok(LockStatus::Done(meta));
        }

        // Not owned by us: delegate to the inner lock.
        match unsafe { self.inner.try_lock(current_iteration) }
        {
            Ok(LockStatus::Done(meta)) =>
            {
                // Store a clone of the metadata for the eventual release. The
                // original is returned to the caller.
                //
                // SAFETY: We just acquired exclusive ownership of the inner
                // lock, so no other thread can be accessing `meta`.
                unsafe { *self.meta.get() = Some(meta.clone()) };
                self.owner.store(id, Ordering::Release);
                self.count.store(1, Ordering::Release);
                Ok(LockStatus::Done(meta))
            },
            Ok(LockStatus::Fail) => Ok(LockStatus::Fail),
            Err(e) => Err(e),
        }
    }

    /// Releases one level of the reentrant lock.
    ///
    /// The inner lock is only released when the last guard is dropped.
    ///
    /// # Safety
    ///
    /// See [`LockPolicy::free`]. The caller must ensure that the current
    /// thread actually holds the lock.
    unsafe fn free(&self, _meta: &Self::Meta)
    {
        let id: usize = T::thread_id().into();
        debug_assert_eq!(
            self.owner.load(Ordering::Acquire),
            id,
            "attempted to release a reentrant lock not owned by the current \
             thread"
        );

        let prev = self.count.fetch_sub(1, Ordering::AcqRel);
        if prev == 1
        {
            // Last guard dropped: clear ownership and release the inner lock.
            // Ownership is cleared *before* calling `inner.free` so that a
            // concurrent acquirer cannot observe a non-zero owner after the
            // inner lock has been released.
            self.owner.store(0, Ordering::Release);

            // SAFETY: We are the owner and this is the final release. No
            // other thread can be accessing `meta`.
            let meta = unsafe { (*self.meta.get()).take() }
                .expect("reentrant lock lost metadata");
            unsafe { self.inner.free(&meta) };
        }
    }

    fn wake_all(&self)
    {
        self.inner.wake_all()
    }

    fn abort(&self)
    {
        self.inner.abort()
    }
}

unsafe impl<L, T> SharingPolicy for Retaint<L, T>
where
    L: SharingPolicy,
    L::Meta: Clone + Send,
    T: StableThreadId,
    T::Id: Copy + Into<usize>,
{
    /// Attempts to acquire a shared lock.
    ///
    /// Shared acquisitions are not made reentrant. If the current thread
    /// already holds the lock exclusively, this method returns
    /// [`LockStatus::Fail`] to prevent unsupported downgrade attempts.
    /// Otherwise the call is delegated to the inner lock.
    fn try_share(
        &self,
        current_iteration: usize,
    ) -> LockResult<Self::Meta, Self::Error>
    {
        let id: usize = T::thread_id().into();
        let owner = self.owner.load(Ordering::Acquire);

        if owner == id && self.count.load(Ordering::Acquire) > 0
        {
            // The current thread owns the lock exclusively. Downgrading to
            // shared access is not supported.
            return Ok(LockStatus::Fail);
        }

        self.inner.try_share(current_iteration)
    }

    /// Releases a shared lock.
    ///
    /// Shared locks are not tracked by the reentrancy counter, so this call is
    /// delegated directly to the inner lock.
    fn free_share(&self, meta: &Self::Meta)
    {
        self.inner.free_share(meta)
    }

    fn wake_readers(&self)
    {
        self.inner.wake_readers()
    }
}
