//! Built-in implementations of [`LockPolicy`](crate::traits::LockPolicy) and
//! [`SharingPolicy`](crate::traits::SharingPolicy).
//!
//! This module provides several ready-to-use lock backends that can be plugged
//! into higher-level primitives like [`Mutex`](crate::Mutex). The available
//! implementations depend on the target operating system and enabled features.

pub(crate) mod atomic;
#[cfg(feature = "fake")]
pub(crate) mod fake;
pub(crate) mod nested;
pub(crate) mod shield;

#[cfg(all(feature = "std", unix, dev))]
mod fs;

pub use atomic::*;
#[cfg(feature = "fake")]
pub use fake::*;
pub use nested::*;
pub use shield::*;

#[cfg(all(feature = "std", unix, dev))]
pub use fs::*;

#[cfg(any(feature = "__lint", all(feature = "std", target_os = "linux")))]
#[cfg_attr(feature = "__lint", doc(hidden))]
pub mod linux;

#[cfg(all(feature = "std", target_os = "linux"))]
pub use linux::*;

#[cfg(any(feature = "__lint", all(feature = "std", target_os = "windows")))]
#[cfg_attr(feature = "__lint", doc(hidden))]
pub mod windows;

#[cfg(all(feature = "std", target_os = "windows"))]
pub use windows::*;

#[cfg(any(feature = "__lint", all(feature = "std", target_os = "macos")))]
#[cfg_attr(feature = "__lint", doc(hidden))]
pub mod macos;

#[cfg(all(feature = "std", target_os = "macos"))]
pub use macos::*;

/// Fallback to the atomic lock strategy when the current OS is not natively
/// supported by Resync's `std` feature.
#[cfg(any(
    feature = "__lint",
    all(
        feature = "std",
        not(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "macos",
        ))
    )
))]
#[cfg_attr(feature = "__lint", doc(hidden))]
pub mod fallback
{
    pub type Os = super::Atomic;
}

#[cfg(all(
    feature = "std",
    not(any(target_os = "linux", target_os = "windows", target_os = "macos",))
))]
pub use fallback::*;
