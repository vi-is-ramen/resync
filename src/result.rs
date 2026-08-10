//! Result types used by locks and spin loops.

/// The outcome of a lock acquisition attempt.
///
/// # Variants
/// - [`LockResult::Done`]  – the lock was successfully acquired.
/// - [`LockResult::Fail`]  – the lock was already held by another owner.
/// - [`LockResult::Abort`] – an unrecoverable error occurred (e.g.,
///   system‑level failure).
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
#[repr(u8)]
pub enum LockResult
{
    /// The lock was successfully acquired.
    Done  = 0,
    /// The lock was already held by another owner.
    Fail  = 1,
    /// An unrecoverable error occurred (e.g., system‑level failure).
    Abort = 2,
}

/// The outcome of a single spin cycle.
///
/// # Variants
/// - [`SpinResult::Ok`]    – the spin completed normally.
/// - [`SpinResult::Abort`] – the spin should be aborted.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
#[repr(u8)]
pub enum SpinResult
{
    /// The spin completed normally.
    Ok    = 1,
    /// The spin should be aborted.
    Abort = 2,
}
