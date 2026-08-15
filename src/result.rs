//! Result types for lock and retry operations.
//!
//! This module defines the core result types used by the lock policies in this
//! crate. They are designed to be flexible, performant, and to clearly
//! distinguish between **transient contention** (the lock is held by someone
//! else) and **fatal errors** (the lock is corrupted or the operation must
//! abort).
//!
//! # Design
//!
//! All lock‑acquisition methods return a [`LockResult<E>`], which is a
//! `Result<LockStatus, E>`. The [`LockStatus`] enum has two variants:
//! - [`LockStatus::Done`] – the lock was successfully acquired.
//! - [`LockStatus::Fail`] – the lock is currently held by another owner.
//!
//! This separation allows the caller to decide how to handle contention
//! (e.g., by retrying with a [`RetryPolicy`]) without conflating it with
//! unrecoverable errors.
//!
//! For retry policies, the [`retry`](RetryPolicy::retry) method returns a
//! [`RetryResult<E>`], which is a `Result<(), E>`. An `Ok(())` means “continue
//! waiting”, while an `Err(e)` means “abort the retry loop and propagate the
//! error”.
//!
//! # Error Types
//!
//! Each lock policy defines its own associated `Error` type. This can be:
//! - [`core::convert::Infallible`] for locks that never fail (e.g., spinlocks).
//! - A custom error type for locks that can fail (e.g., due to poisoning,
//!   timeouts, or resource unavailability).
//!
//! # Performance
//!
//! When the error type is [`Infallible`], the Rust compiler optimises
//! `Result<T, Infallible>` to just `T` (via the `Try` trait and niche
//! optimisation). This means that lock operations that never fail incur
//! **zero runtime overhead** for error handling – they are as efficient as
//! returning a plain `LockStatus`.
//!
//! # Examples
//!
//! A typical lock acquisition loop using these result types:
//!
//! ```no_run
//! # use resync::{LockResult, LockStatus};
//! # fn try_lock() -> LockResult<std::io::Error> { Ok(LockStatus::Done) }
//! # fn retry() -> Result<(), std::io::Error> { Ok(()) }
//! # let mut iteration = 0;
//! loop
//! {
//!     match try_lock()?
//!     {
//!         LockStatus::Done => break,
//!         LockStatus::Fail =>
//!         {
//!             // Retry with a policy.
//!             retry().map_err(|e| e)?;
//!             iteration += 1;
//!         },
//!     }
//! }
//! # Ok::<_, std::io::Error>(())
//! ```
//!
//! # See Also
//!
//! - [`LockPolicy`] – the trait that uses [`LockResult`] for exclusive locks.
//! - [`SharingPolicy`] – the trait that uses [`LockResult`] for shared locks.
//! - [`RetryPolicy`] – the trait that uses [`RetryResult`].
//! - [`core::convert::Infallible`] – the never‑fails error type.

/// The status of a lock acquisition attempt.
///
/// This enum distinguishes between successful acquisition and contention,
/// allowing callers to make informed decisions about retry strategies.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[repr(u8)]
pub enum LockStatus
{
    /// The lock was already held by another owner.
    Fail = 0,
    /// The lock was successfully acquired.
    Done = 1,
}

/// Result type for lock acquisition operations.
///
/// - `Ok(LockStatus::Done)`: lock acquired successfully
/// - `Ok(LockStatus::Fail)`: lock is held by another owner
/// - `Err(E)`: unrecoverable error or operation aborted
///
/// The error type `E` is determined by the lock implementation. For locks
/// that never fail, use `core::convert::Infallible`.
pub type LockResult<E = core::convert::Infallible> = Result<LockStatus, E>;

/// Result type for retry operations.
///
/// - `Ok(())`: retry completed, continue waiting
/// - `Err(E)`: retry aborted (timeout, error, etc.)
///
/// The error type `E` is determined by the retry implementation.
pub type RetryResult<E = core::convert::Infallible> = Result<(), E>;

/// Error returned by non‑blocking lock acquisition attempts (e.g., `try_lock`).
///
/// This error distinguishes between transient contention and fatal lock errors.
#[derive(Debug)]
pub enum TryLockError<E>
{
    /// The lock is currently held by another owner (or writer).
    Contention,
    /// An unrecoverable error occurred in the underlying lock policy.
    Lock(E),
}

/// Error returned by blocking lock acquisition attempts (e.g., `lock`) when
/// the retry loop aborts or the lock policy fails.
#[derive(Debug)]
pub enum LockError<LE, RE>
{
    /// An unrecoverable error occurred in the underlying lock policy.
    Lock(LE),
    /// The retry policy aborted the acquisition loop (e.g., due to a timeout).
    Retry(RE),
}
