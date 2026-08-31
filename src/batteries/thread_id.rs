//! Default thread identifier providers.
//!
//! This module provides [`DefaultThreadId`], the default implementation of
//! [`StableThreadId`](crate::api::StableThreadId) used by the reentrant lock
//! wrapper.
//!
//! Under the `std` feature it uses the address of a thread-local variable
//! as a unique, stable identifier. In `#![no_std]` builds this type exists
//! but does **not** implement the trait; you must supply your own
//! implementation.

#[cfg(std)]
use crate::api::StableThreadId;

/// Default thread identifier provider.
///
/// Under the `std` feature this uses the address of a thread-local variable
/// as a unique, stable identifier. In `#![no_std]` builds this type exists
/// but does **not** implement [`StableThreadId`]; you must supply your own
/// implementation.
///
/// [`StableThreadId`]: crate::api::StableThreadId
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultThreadId;

#[cfg(std)]
unsafe impl StableThreadId for DefaultThreadId
{
    type Id = usize;

    fn thread_id() -> usize
    {
        thread_local! {
            #[allow(clippy::missing_const_for_thread_local)]
            static ID: usize = 0;
        }

        ID.with(|id| id as *const usize as usize)
    }
}
