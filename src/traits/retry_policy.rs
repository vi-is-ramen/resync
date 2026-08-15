use crate::RetryResult;

/// A policy for retrying a lock acquisition when it is not immediately
/// available.
///
/// This trait abstracts the waiting or spinning strategy used by a lock when a
/// [`try_lock`](LockPolicy::try_lock) or
/// [`try_share`](SharingPolicy::try_share) call returns [`LockStatus::Fail`].
/// Instead of deciding the waiting strategy inside the lock itself, the lock
/// delegates to a `RetryPolicy`, which can be swapped to implement different
/// backoff, spin, yield, or parking behaviours.
///
/// # Purpose
///
/// Lock implementations often need to wait for the lock to become free. The
/// waiting strategy can significantly affect performance and fairness:
/// - Spinning (busy‑waiting) is cheap but wastes CPU cycles if the wait is
///   long.
/// - Yielding gives up the current time‑slice and is more cooperative.
/// - Parking (blocking) puts the thread to sleep, saving CPU but adding
///   latency.
///
/// By encapsulating the retry logic in a separate policy, the lock can be
/// parameterised with different behaviours at compile time or runtime, allowing
/// fine‑tuning for different workloads (e.g., low‑contention vs.
/// high‑contention).
///
/// # Associated Types
///
/// * `Error` – The error type for unrecoverable failures that should abort the
///   retry loop. Implementations that never abort can use [`Infallible`].
///
/// # Required Super‑trait
///
/// `Self: Default` – The retry policy must be default‑constructible, typically
/// representing a sensible default waiting strategy (e.g., a short spin loop).
///
/// # Adaptive Behaviour
///
/// The [`retry`](#tymethod.retry) method receives a `current_iteration`
/// parameter, which counts how many times the lock acquisition has been tried
/// (including this call). Implementations may use this to adapt their
/// behaviour:
/// - Spin for the first N iterations.
/// - Yield the thread for the next M iterations.
/// - Park (block) after a threshold.
///
/// This allows the policy to implement efficient adaptive synchronisation,
/// similar to the approach used in many operating system kernels.
///
/// # Errors and Aborting
///
/// The [`retry`](#tymethod.retry) method returns a [`RetryResult`] (which is
/// analogous to [`LockResult`]) that can either indicate that waiting should
/// continue (`Ok(())`) or that the retry loop should be aborted because an
/// unrecoverable error has occurred (`Err(e)`). Such errors might include:
/// - The lock is poisoned (the holder panicked).
/// - The underlying resource is permanently unavailable (e.g., a file was
///   deleted, or a network connection died).
///
/// Note that a return value of `Err(e)` **does not** mean that the lock
/// acquisition was successful; it means the retry loop must stop and propagate
/// the error to the caller. In this sense, the error is a *fatal* condition
/// that prevents further attempts.
///
/// # Examples
///
/// A simple spin policy that always busy‑waits without yielding:
///
/// ```rust
/// # use core::convert::Infallible;
/// # use core::hint::spin_loop;
/// # use resync::traits::RetryPolicy;
/// # use resync::RetryResult;
/// #[derive(Default)]
/// struct SpinPolicy;
///
/// impl RetryPolicy for SpinPolicy
/// {
///     type Error = Infallible;
///
///     fn retry(&self, _: usize) -> RetryResult<Self::Error>
///     {
///         spin_loop();
///         Ok(())
///     }
/// }
/// ```
///
/// A more advanced policy that spins for 100 iterations, then yields:
///
/// ```rust
/// # use core::convert::Infallible;
/// # use core::hint::spin_loop;
/// # use std::thread::yield_now;
/// # use resync::traits::RetryPolicy;
/// # use resync::RetryResult;
/// #[derive(Default)]
/// struct AdaptivePolicy;
///
/// impl RetryPolicy for AdaptivePolicy
/// {
///     type Error = Infallible;
///
///     fn retry(&self, current_iteration: usize) -> RetryResult<Self::Error>
///     {
///         if current_iteration < 100
///         {
///             spin_loop();
///         }
///         else
///         {
///             yield_now();
///         }
///         Ok(())
///     }
/// }
/// ```
///
/// A policy that aborts after a timeout (using a hypothetical `Instant`):
///
/// ```rust
/// # use core::convert::Infallible;
/// # use resync::traits::RetryPolicy;
/// # use resync::RetryResult;
/// # use std::time::{Duration, Instant};
/// #
/// struct TimeoutPolicy
/// {
///     start:   Instant,
///     timeout: Duration,
/// }
///
/// impl Default for TimeoutPolicy
/// {
///     fn default() -> Self
///     {
///         Self {
///             start:   Instant::now(),
///             timeout: Duration::from_secs(1),
///         }
///     }
/// }
///
/// impl RetryPolicy for TimeoutPolicy
/// {
///     type Error = std::io::Error; // Or a custom error type.
///
///     fn retry(&self, _: usize) -> RetryResult<Self::Error>
///     {
///         if self.start.elapsed() > self.timeout
///         {
///             // Abort with an error.
///             Err(std::io::Error::new(
///                 std::io::ErrorKind::TimedOut,
///                 "lock acquisition timed out",
///             ))
///         }
///         else
///         {
///             std::thread::yield_now();
///             Ok(())
///         }
///     }
/// }
/// ```
///
/// # See Also
///
/// - [`LockPolicy`] and [`SharingPolicy`] – the lock policies that use this
///   retry policy.
/// - [`RetryResult`] – the return type of [`retry`](#tymethod.retry), analogous
///   to [`LockResult`].
/// - [`core::convert::Infallible`] – for error types that can never occur.
pub trait RetryPolicy
where Self: Default
{
    /// The error type for unrecoverable failures that abort the retry loop.
    ///
    /// Use [`Infallible`] for policies that never abort.
    type Error;

    /// Perform one retry iteration.
    ///
    /// This method is called by the lock when a previous acquisition attempt
    /// failed. The lock will invoke this method repeatedly until either:
    /// - The lock becomes available and the acquisition succeeds (handled by
    ///   the lock itself, not by this method).
    /// - This method returns `Err(e)`, indicating that the retry loop should be
    ///   aborted due to a fatal error.
    /// - The lock is acquired by another thread? Actually, the lock's retry
    ///   loop will call `try_lock`/`try_share` each time; if that succeeds, the
    ///   loop ends; otherwise it calls `retry` again.
    ///
    /// # Returns
    ///
    /// - `Ok(())`: continue retrying; the lock will attempt to acquire again.
    /// - `Err(e)`: abort the retry loop immediately and propagate `e` to the
    ///   caller of the lock acquisition function.
    ///
    /// # Implementation Notes
    ///
    /// The method should *never* attempt to acquire the lock itself. Its sole
    /// responsibility is to decide what to do *while waiting*: spin, yield,
    /// park, or abort. The actual acquisition attempt is performed by the lock
    /// after each call to `retry`.
    ///
    /// It is important that the method does not block indefinitely unless that
    /// is the intended behaviour (e.g., parking). However, parking should be
    /// used with care, as the lock must then provide a way to wake the thread
    /// (e.g., via [`LockPolicy::wake_all`] or [`SharingPolicy::wake_readers`]).
    ///
    /// # Example
    ///
    /// ```
    /// # use resync::traits::RetryPolicy;
    /// # use core::hint::spin_loop;
    /// # use resync::RetryResult;
    /// # use core::convert::Infallible;
    /// # struct MyPolicy;
    /// # impl Default for MyPolicy { fn default() -> Self { MyPolicy } }
    /// # impl RetryPolicy for MyPolicy {
    /// #   type Error = Infallible;
    /// fn retry(&self, current_iteration: usize) -> RetryResult<Self::Error>
    /// {
    ///     if current_iteration < 10
    ///     {
    ///         spin_loop(); // busy-wait for first 10 tries
    ///     }
    ///     else
    ///     {
    ///         std::thread::yield_now(); // then yield
    ///     }
    ///     Ok(())
    /// }
    /// # }
    /// ```
    fn retry(&self, current_iteration: usize) -> RetryResult<Self::Error>;
}
