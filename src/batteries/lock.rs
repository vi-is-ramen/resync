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
#[cfg_attr(feature = "__lint", allow(dead_code))]
pub(crate) mod linux;

#[cfg(all(feature = "std", target_os = "linux"))]
#[cfg_attr(
    feature = "__lint",
    allow(ambiguous_glob_reexports, unused_imports, dead_code)
)]
pub use linux::*;

#[cfg(any(feature = "__lint", all(feature = "std", target_os = "windows")))]
#[cfg_attr(
    feature = "__lint",
    allow(ambiguous_glob_reexports, unused_imports, dead_code)
)]
pub(crate) mod windows;

#[cfg(all(feature = "std", target_os = "windows"))]
#[cfg_attr(
    feature = "__lint",
    allow(ambiguous_glob_reexports, unused_imports, dead_code)
)]
pub use windows::*;

#[cfg(any(feature = "__lint", all(feature = "std", target_os = "macos")))]
#[cfg_attr(feature = "__lint", allow(dead_code))]
pub(crate) mod macos;

#[cfg(all(feature = "std", target_os = "macos"))]
#[cfg_attr(
    feature = "__lint",
    allow(ambiguous_glob_reexports, unused_imports, dead_code)
)]
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
pub(crate) mod fallback
{
    /// Fallback `Os` lock variant.
    pub type Os = super::Atomic;
}

#[cfg(any(
    feature = "__lint",
    not(all(
        feature = "std",
        any(target_os = "linux", target_os = "windows", target_os = "macos",)
    ))
))]
#[cfg_attr(
    feature = "__lint",
    allow(ambiguous_glob_reexports, unused_imports, dead_code)
)]
pub use fallback::*;

#[cfg(any(feature = "__lint", not(feature = "std")))]
pub(crate) mod irq;

#[cfg(any(feature = "__lint", not(feature = "std")))]
pub use irq::*;
