//! Result types used by locks and spin loops.
//!
//! # Design
//!
//! This module provides flexible, performance‑oriented result types using
//! Rust's `Result` type with associated error types:
//!
//! - **`LockStatus`**: enum distinguishing successful acquisition from failure
//! - **`LockResult<E>`**: `Result<LockStatus, E>` for lock operations
//! - **`SpinResult<E>`**: `Result<(), E>` for spin operations
//!
//! # Error Types
//!
//! Each trait (`ILock`, `IShare`, `ISpin`) has an associated `Error` type:
//! - For operations that never fail: use `core::convert::Infallible`
//! - For operations with specific errors: define custom error types
//!
//! # Performance
//!
//! When `E = Infallible`, the compiler optimizes `Result<T, Infallible>` to
//! just `T`, eliminating any overhead.
//!
//! # Examples
//!
//! ```rust
//! use resync::lock::Atomic;
//! use resync::{ILock, LockResult, LockStatus};
//!
//! let lock = Atomic::new();
//!
//! match lock.try_lock(0)
//! {
//!     Ok(LockStatus::Done) => println!("Acquired!"),
//!     Ok(LockStatus::Fail) => println!("Lock held"),
//!     Err(e) => println!("Error: {:?}", e),
//! }
//! ```

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

/// Result type for spin operations.
///
/// - `Ok(())`: spin completed, continue waiting
/// - `Err(E)`: spin aborted (timeout, error, etc.)
///
/// The error type `E` is determined by the spin implementation.
pub type SpinResult<E = core::convert::Infallible> = Result<(), E>;
