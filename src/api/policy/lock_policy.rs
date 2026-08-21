#![allow(unused_imports)]
use crate::{LockResult, LockStatus};
use core::convert::Infallible;

/// A lock policy that defines how to acquire, release, and inspect a lock.
///
/// This trait abstracts the behavior of a synchronization primitive (e.g., a
/// mutex, spinlock, futex, or distributed lock) in terms of three core
/// operations: non‑blocking acquisition, release, and state inspection. It is
/// designed to be used as the policy parameter of a generic lock type, allowing
/// the same lock interface to be backed by different implementations.
///
/// For locks that can be initialized in the acquired state, see the
/// [`NewLocked`](crate::traits::NewLocked) extension trait.
///
/// # Safety
///
/// This trait is **unsafe** to implement because the implementor must guarantee
/// that the lock is correctly synchronized and that all memory ordering
/// requirements are satisfied. Incorrect implementations can lead to data
/// races, deadlocks, or corrupted state. Specifically:
///
/// - [`try_lock`](#tymethod.try_lock) must observe or update the lock state
///   atomically and with appropriate barriers.
/// - [`free`](#tymethod.free) must only be called when the current thread
///   actually holds the lock; failure to uphold this invariant may release a
///   lock not owned by the caller.
/// - [`get_state`](#tymethod.get_state) must not modify the lock state and must
///   be safe to call concurrently from any thread.
/// - [`wake_all`](#tymethod.wake_all), if overridden, must ensure that all
///   waiters are woken correctly without race conditions.
///
/// # Associated Types
///
/// * `Error` – The error type for unrecoverable failures, such as a poisoned
///   lock (after a panic) or a resource becoming permanently unavailable.
///   Implementations that never fail can use [`Infallible`].
/// * `Meta` – Metadata returned upon successful acquisition that must be passed
///   back to [`free`](#tymethod.free) to release the lock.
///
/// # Required Super‑traits
///
/// `Self: Sync` – The lock policy must be safely shared.
///
/// # Adaptive Behaviour
///
/// The [`try_lock`](#tymethod.try_lock) method receives a `current_iteration`
/// parameter, which counts how many times the caller has already attempted to
/// acquire the lock. Implementations may use this to adapt their waiting
/// strategy - for example, spinning for a few iterations before parking the
/// thread. This enables efficient adaptive synchronisation.
///
/// # Idempotent Release
///
/// The [`free`](#tymethod.free) method is **idempotent**: calling it multiple
/// times is safe (though only the first call actually releases the lock). This
/// allows implementations to be forgiving in error‑handling paths.
///
/// # Examples
///
/// A simple spinlock policy that never parks:
///
/// ```rust
/// # use core::convert::Infallible;
/// # use resync::api::LockPolicy;
/// # use resync::LockResult;
/// # use resync::LockStatus;
/// # use core::sync::atomic::{AtomicBool, Ordering};
///
/// #[repr(transparent)]
/// struct SpinPolicy(AtomicBool);
///
/// impl Default for SpinPolicy
/// {
///     fn default() -> Self
///     {
///         SpinPolicy(AtomicBool::new(false))
///     }
/// }
///
/// unsafe impl LockPolicy for SpinPolicy
/// {
///     type Error = Infallible;
///     type Meta = ();
///
///     unsafe fn try_lock(
///         &self,
///         _: usize,
///     ) -> LockResult<Self::Meta, Self::Error>
///     {
///         // Note: A production policy might spin for the first 100 iterations
///         // and then call `std::thread::yield_now()` or park.
///
///         let was_locked = self.0.swap(true, Ordering::Acquire);
///         if was_locked
///         {
///             Ok(LockStatus::Fail)
///         }
///         else
///         {
///             Ok(LockStatus::Done(()))
///         }
///     }
///
///     unsafe fn free(&self, _: &())
///     {
///         self.0.store(false, Ordering::Release);
///     }
/// }
/// ```
///
/// # See Also
///
/// - The [`NewLocked`](crate::traits::NewLocked) trait for locked
///   initialization.
/// - The [`LockResult`] type and the [`LockStatus`] enum used in method return
///   values.
// NOTE: This trait **must** be dyn-compatible by design.
pub unsafe trait LockPolicy
where Self: Sync
{
    /// The error type for unrecoverable failures.
    ///
    /// Use [`Infallible`] for locks that never fail.
    type Error: core::error::Error;

    /// Metadata associated with a successful lock acquisition.
    ///
    /// This type is returned by [`try_lock`](#tymethod.try_lock) upon
    /// successful acquisition ([`LockStatus::Done`]) and must be passed
    /// back to the [`free`](#tymethod.free) method to release the lock.
    ///
    /// For simple locks (like basic spinlocks), this can be the unit type `()`.
    /// For more complex locks (like ticket locks, OS futexes, or locks that
    /// track the owner thread), this type carries the necessary state to
    /// correctly identify and release the specific lock instance.
    type Meta;

    /// Attempt to acquire the lock.
    ///
    /// The `current_iteration` parameter indicates how many times the caller
    /// has already attempted to acquire the lock. Implementations may use
    /// this to decide whether to park the current thread when the iteration
    /// count exceeds some threshold. It's the base for adaptive
    /// synchronization.
    ///
    /// # Returns
    ///
    /// - [`LockStatus::Done`]: acquisition is successful;
    /// - [`LockStatus::Fail`]: Lock is acquired already.
    ///
    /// # Errors
    ///
    /// This method will return [`Self::Error`] error if the lock is corrupted.
    /// Examples:
    /// - thread holding the lock had panicked (poisonous lock);
    /// - resource is no longer available (if the lock relies on a network or
    ///   filesystem);
    /// - and so on.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the implementor must guarantee atomicity
    /// and proper memory ordering (e.g., using Acquire/Release fences). Callers
    /// must not rely on the return value to uphold safety invariants without
    /// holding the lock, as the state may change between the check and their
    /// subsequent action.
    unsafe fn try_lock(
        &self,
        current_iteration: usize,
    ) -> LockResult<Self::Meta, Self::Error>;

    /// Release the lock.
    ///
    /// This method is idempotent. For futex‑based implementations, it may
    /// wake one waiting thread.
    ///
    /// # Safety
    ///
    /// This method is unsafe because a faulty implementation could fail to
    /// enforce the necessary memory ordering (e.g., missing Acquire/Release
    /// barriers) or fail to handle compiler reordering correctly, leading to
    /// data races on the protected data. The implementor must guarantee
    /// atomicity.
    unsafe fn free(&self, meta: &Self::Meta);

    /// Wake all threads waiting on this lock.
    ///
    /// The default implementation is a no‑op. Futex‑based (or similar)
    /// implementations should override this to broadcast a wake to all
    /// waiters.
    fn wake_all(&self) {}

    /// What to do if exclusive acquisition loop was aborted?
    ///
    /// The default implementation is no-op.
    fn abort(&self) {}
}
