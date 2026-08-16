#![allow(type_alias_bounds)]
//! Result types for lock and retry operations.
//!
//! This module defines the core result types used by the lock policies in this
//! crate. They are designed to be flexible, performant, and to clearly
//! distinguish between **transient contention** (the lock is held by someone
//! else), **fatal errors** (the lock is corrupted or the operation must
//! abort), and **poisoned state** (a previous thread panicked while holding
//! the lock).

/// The status of a lock acquisition attempt.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum LockStatus<M>
{
    /// The lock was already held by another owner.
    Fail,
    /// The lock was successfully acquired.
    Done(M),
}

/// Result type for raw lock acquisition operations (used by `LockPolicy`).
pub type LockResult<M, E>
where E: core::error::Error
= Result<LockStatus<M>, E>;

/// Result type for retry operations.
pub type RetryResult<E>
where E: core::error::Error
= Result<(), E>;

/// An error returned when a lock is poisoned.
///
/// A lock becomes poisoned when a thread panics while holding the lock.
/// This error contains the guard, allowing the caller to inspect or
/// recover the potentially inconsistent data by calling
/// [`into_inner`](Self::into_inner).
#[derive(Debug)]
pub struct PoisonError<T>
{
    guard: T,
}

impl<T> PoisonError<T>
{
    /// Creates a new `PoisonError` wrapping the given guard.
    pub fn new(guard: T) -> Self
    {
        Self { guard }
    }

    /// Consumes this error, returning the underlying guard.
    ///
    /// This allows access to the protected data even if the lock is poisoned,
    /// enabling manual recovery or inspection of the inconsistent state.
    pub fn into_inner(self) -> T
    {
        self.guard
    }
}

impl<T> core::fmt::Display for PoisonError<T>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.write_str("lock poisoned")
    }
}

impl<T: core::fmt::Debug> core::error::Error for PoisonError<T> {}

/// Error returned by blocking lock acquisition attempts (e.g., `lock`, `read`,
/// `write`).
///
/// This error distinguishes between a poisoned lock, a fatal lock policy
/// error, and a retry policy abortion.
#[derive(Debug)]
pub enum AcquireError<Guard, LE, RE>
where
    LE: core::error::Error,
    RE: core::error::Error,
{
    /// The lock was poisoned because a previous thread panicked while
    /// holding it. The guard is provided to allow data recovery.
    Poisoned(PoisonError<Guard>),
    /// An unrecoverable error occurred in the underlying lock policy.
    Lock(LE),
    /// The retry policy aborted the acquisition loop (e.g., due to a timeout).
    Retry(RE),
}

impl<Guard, LE, RE> core::fmt::Display for AcquireError<Guard, LE, RE>
where
    LE: core::error::Error + core::fmt::Display,
    RE: core::error::Error + core::fmt::Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        match self
        {
            Self::Poisoned(e) =>
            {
                <PoisonError<Guard> as core::fmt::Display>::fmt(e, f)
            },
            Self::Lock(e) => <LE as core::fmt::Display>::fmt(e, f),
            Self::Retry(e) => <RE as core::fmt::Display>::fmt(e, f),
        }
    }
}

impl<Guard, LE, RE> core::error::Error for AcquireError<Guard, LE, RE>
where
    Guard: core::fmt::Debug + 'static,
    LE: core::error::Error + 'static,
    RE: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)>
    {
        match self
        {
            Self::Poisoned(e) => Some(e),
            Self::Lock(e) => e.source(),
            Self::Retry(e) => e.source(),
        }
    }
}

/// Error returned by non‑blocking lock acquisition attempts (e.g., `try_lock`).
#[derive(Debug)]
pub enum TryLockError<Guard, LE>
where LE: core::error::Error
{
    /// The lock is currently held by another owner (or writer).
    Contention,
    /// An unrecoverable error occurred in the underlying lock policy.
    Lock(LE),
    /// The lock was poisoned because a previous thread panicked while
    /// holding it. The guard is provided to allow data recovery.
    Poisoned(PoisonError<Guard>),
}

impl<Guard, LE> core::fmt::Display for TryLockError<Guard, LE>
where LE: core::error::Error + core::fmt::Display
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        match self
        {
            Self::Contention => f.write_str("lock contention"),
            Self::Lock(e) => <LE as core::fmt::Display>::fmt(e, f),
            Self::Poisoned(e) =>
            {
                <PoisonError<Guard> as core::fmt::Display>::fmt(e, f)
            },
        }
    }
}

impl<Guard, LE> core::error::Error for TryLockError<Guard, LE>
where
    Guard: core::fmt::Debug + 'static,
    LE: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)>
    {
        match self
        {
            Self::Contention => None,
            Self::Lock(e) => e.source(),
            Self::Poisoned(e) => Some(e),
        }
    }
}
