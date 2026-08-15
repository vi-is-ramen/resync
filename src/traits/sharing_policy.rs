#![allow(unused_imports)]

use crate::traits::LockPolicy;
use crate::{LockResult, LockStatus};
use core::convert::Infallible;

/// A lock policy that extends exclusive locking with shared (reader) access.
///
/// This trait builds upon [`LockPolicy`] to support read‑write locks, where
/// multiple threads may hold the lock concurrently for reading, but only one
/// thread may hold it for writing (exclusive). The policy defines the core
/// operations for acquiring and releasing shared (reader) locks.
///
/// # Purpose
///
/// Read‑write locks are useful when shared data is frequently read but
/// infrequently modified. By allowing concurrent readers, they can provide
/// better performance than a simple mutex in read‑heavy workloads.
///
/// # Associated Methods
///
/// - [`try_share`](#tymethod.try_share) – attempt to acquire a shared (reader)
///   lock.
/// - [`free_share`](#tymethod.free_share) – release a previously acquired
///   reader lock.
/// - [`wake_readers`](#tymethod.wake_readers) – wake all threads waiting for a
///   reader lock (optional).
///
/// # Relationship to [`LockPolicy`]
///
/// `SharingPolicy` inherits all the exclusive‑lock operations from
/// [`LockPolicy`]:
/// - [`LockPolicy::try_lock`] acquires the lock for exclusive (writer) access.
/// - [`LockPolicy::free`] releases an exclusive (writer) lock.
/// - [`LockPolicy::get_state`] inspects the lock state.
/// - [`LockPolicy::wake_all`] wakes all threads waiting for an exclusive
///   (writer) lock.
///
/// The shared operations are *in addition* to these, allowing a single lock
/// primitive to support both reader and writer access.
///
/// # Safety
///
/// This trait is **unsafe** to implement because the implementor must guarantee
/// correct synchronisation for both shared and exclusive access. Specifically:
///
/// - [`try_share`](#tymethod.try_share) must observe or update the reader‑count
///   atomically with appropriate memory barriers. It must ensure that readers
///   are allowed to proceed when no writer holds the lock, and that writers are
///   blocked while any reader holds the lock.
/// - [`free_share`](#tymethod.free_share) must only be called when the current
///   thread actually holds a reader lock; failure to uphold this invariant may
///   release a reader lock not owned by the caller, corrupting the reader
///   count.
/// - The combination of [`LockPolicy::try_lock`] (writer) and
///   [`try_share`](#tymethod.try_share) (reader) must be mutually exclusive: a
///   writer must not be able to acquire the lock while any reader holds it, and
///   a reader must not be able to acquire the lock while a writer holds it.
/// - [`wake_readers`](#tymethod.wake_readers), if overridden, must ensure that
///   all waiter threads are woken correctly without race conditions.
///
/// # Associated Types
///
/// * `Error` – The error type for unrecoverable failures, same as in
///   [`LockPolicy`]. Implementations that never fail can use [`Infallible`].
///
/// # Required Super‑trait
///
/// `Self: LockPolicy + Default` – Every sharing policy must also implement the
/// exclusive‑lock policy and be default‑constructible.
///
/// # Adaptive Behaviour
///
/// Like [`LockPolicy::try_lock`], the [`try_share`](#tymethod.try_share) method
/// receives a `current_iteration` parameter that counts how many times the
/// caller has already attempted to acquire the lock. Implementations may use
/// this to adapt their waiting strategy – for example, spinning for the first
/// few iterations, yielding the thread, or parking. This enables efficient
/// adaptive synchronisation for reader locks as well.
///
/// # Idempotent Release
///
/// The [`free_share`](#tymethod.free_share) method is **idempotent**: calling
/// it multiple times is safe (though only the first call actually releases the
/// reader lock). This allows implementations to be forgiving in error‑handling
/// paths. Note that this matches the idempotence guarantee of
/// [`LockPolicy::free`].
///
/// # Examples
///
/// A simple reader‑writer spinlock policy that never parks:
///
/// ```rust
/// # use core::convert::Infallible;
/// # use core::sync::atomic::{AtomicUsize, Ordering};
/// # use resync::traits::LockPolicy;
/// # use resync::LockResult;
/// # use resync::LockStatus;
/// # use resync::traits::SharingPolicy;
/// #[repr(transparent)]
/// struct RwSpinPolicy(AtomicUsize);
///
/// impl Default for RwSpinPolicy
/// {
///     fn default() -> Self
///     {
///         // 0 = unlocked, >0 = readers, usize::MAX = writer
///         RwSpinPolicy(AtomicUsize::new(0))
///     }
/// }
///
/// unsafe impl LockPolicy for RwSpinPolicy
/// {
///     type Error = Infallible;
///
///     unsafe fn try_lock(&self, _: usize) -> LockResult<Self::Error>
///     {
///         // Try to acquire writer lock: swap to usize::MAX if unlocked.
///         // Note: In production, you'd spin for a while before parking.
///         let state = self.0.compare_exchange(
///             0,
///             usize::MAX,
///             Ordering::Acquire,
///             Ordering::Relaxed,
///         );
///         match state
///         {
///             Ok(_) => Ok(LockStatus::Done),
///             Err(_) => Ok(LockStatus::Fail),
///         }
///         .into()
///     }
///
///     unsafe fn free(&self)
///     {
///         // Release writer lock: store 0.
///         self.0.store(0, Ordering::Release);
///     }
///
///     fn get_state(&self) -> LockResult<Self::Error>
///     {
///         let state = self.0.load(Ordering::Relaxed);
///         if state == 0
///         {
///             Ok(LockStatus::Done)
///         }
///         else
///         {
///             Ok(LockStatus::Fail)
///         }
///     }
/// }
///
/// unsafe impl SharingPolicy for RwSpinPolicy
/// {
///     fn try_share(&self, _: usize) -> LockResult<Self::Error>
///     {
///         // Attempt to increment reader count if not held by a writer.
///         // This is a simplified loop; a production implementation would
///         // use a more robust CAS strategy with spinning.
///         let mut current = self.0.load(Ordering::Relaxed);
///         loop
///         {
///             if current == usize::MAX
///             {
///                 // Writer holds the lock, cannot acquire reader lock.
///                 return Ok(LockStatus::Fail).into();
///             }
///             match self.0.compare_exchange(
///                 current,
///                 current + 1,
///                 Ordering::Acquire,
///                 Ordering::Relaxed,
///             )
///             {
///                 Ok(_) => return Ok(LockStatus::Done).into(),
///                 Err(updated) => current = updated,
///             }
///         }
///     }
///
///     fn free_share(&self)
///     {
///         // Decrement reader count.
///         self.0.fetch_sub(1, Ordering::Release);
///     }
///
///     fn wake_readers(&self)
///     {
///         // No-op for this spinlock implementation.
///     }
/// }
/// ```
///
/// # See Also
///
/// - [`LockPolicy`] for the base exclusive‑lock operations.
/// - [`LockResult`] and [`LockStatus`] for return types.
/// - [`Infallible`] for error types that never occur.
pub unsafe trait SharingPolicy: LockPolicy
{
    /// Attempt to acquire the lock for reading (shared access).
    ///
    /// Multiple readers may hold the lock concurrently, but if a writer holds
    /// the lock, this method must fail (or block – the decision to block is
    /// left to the implementation's adaptive strategy).
    ///
    /// The `current_iteration` parameter indicates how many times the caller
    /// has already attempted to acquire the lock. Implementations may use
    /// this to decide whether to park the current thread when the iteration
    /// count exceeds some threshold.
    ///
    /// # Returns
    ///
    /// - [`LockStatus::Done`]: Shared (reader) acquisition is successful.
    /// - [`LockStatus::Fail`]: The lock is currently held by a writer, or
    ///   another condition prevents acquiring a reader lock.
    ///
    /// # Errors
    ///
    /// This method will return an error if the lock is corrupted. Examples:
    /// - the lock has been poisoned (e.g., a previous writer panicked);
    /// - the underlying resource is no longer available (if the lock relies on
    ///   a network or filesystem);
    /// - and so on.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it must correctly synchronise with both
    /// other readers and writers. Failure to use appropriate memory ordering
    /// (e.g., Acquire/Release semantics) may lead to data races on the
    /// protected data. Implementors must ensure that the reader‑count update
    /// is atomic and that the lock state is inspected without TOCTOU races.
    fn try_share(&self, current_iteration: usize) -> LockResult<Self::Error>;

    /// Release a previously acquired shared (reader) lock.
    ///
    /// This method is idempotent: calling it multiple times is safe (though
    /// only the first call actually decrements the reader count).
    ///
    /// After releasing the last reader, the lock becomes available for
    /// either readers or writers. For futex‑based implementations, this
    /// method may wake one waiting writer (or all waiting threads) depending
    /// on the scheduling policy.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it must only be called when the current
    /// thread actually holds a reader lock. Calling it without holding a
    /// reader lock may corrupt the reader count or release a lock not owned
    /// by the caller. Implementors must ensure that the decrement is atomic
    /// and uses appropriate memory barriers (e.g., Release ordering) to
    /// guarantee that all reads performed under the lock are visible to
    /// subsequent writers.
    fn free_share(&self);

    /// Wake all threads waiting for a shared (reader) lock.
    ///
    /// The default implementation is a no‑op. Futex‑based (or similar)
    /// implementations should override this to broadcast a wake to all
    /// threads that are blocked on [`try_share`](#tymethod.try_share).
    ///
    /// This method is typically called when the lock transitions from a state
    /// where readers were blocked (e.g., a writer just released the lock) to
    /// a state where readers can proceed.
    ///
    /// # Safety
    ///
    /// This method is safe (no `unsafe` marker) because it does not modify
    /// the lock state itself; however, implementors must ensure that it does
    /// not introduce race conditions with concurrent lock operations.
    fn wake_readers(&self) {}
}
