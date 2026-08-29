//! A trait for obtaining a stable identifier of the current thread.
//!
//! This module provides the [`StableThreadId`] trait, which abstracts the
//! mechanism used to identify the thread that currently owns a reentrant
//! lock.
//!
//! # Purpose
//!
//! Reentrant locks need to know which thread already holds the lock so that
//! recursive acquisitions by the same thread can succeed immediately. In
//! `std` environments this can be done with thread-local storage, but in
//! `#![no_std]` environments users must provide their own implementation.

/// A policy for obtaining a stable identifier of the current thread.
///
/// Implementations must return an identifier that is unique among all live
/// threads and remains stable for the entire lifetime of each thread.
///
/// # Safety
///
/// The implementor must guarantee that identifiers are unique among all live
/// threads. When the identifier is converted to `usize` (as required by
/// reentrant lock wrappers), the conversion must be injective and must never
/// produce `0`, because `0` is reserved for the “unowned” state. Violating
/// this may break lock ownership checks and cause data races.
// NOTE: This trait is not dyn-compatible by design.
pub unsafe trait StableThreadId
{
    /// The thread identifier type.
    type Id: Eq;

    /// Returns the identifier of the current thread.
    fn thread_id() -> Self::Id;
}
