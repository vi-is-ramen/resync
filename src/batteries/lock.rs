//! Built-in implementations of [`LockPolicy`](crate::traits::LockPolicy) and
//! [`SharingPolicy`](crate::traits::SharingPolicy).
//!
//! This module provides several ready-to-use lock backends that can be plugged
//! into higher-level primitives like [`Mutex`](crate::Mutex). The available
//! implementations depend on the target operating system and enabled features.

mod atomic;
mod nested;

pub use atomic::*;
pub use nested::*;

#[cfg(all(feature = "std", target_os = "linux"))]
mod linux;

#[cfg(all(feature = "std", target_os = "linux"))]
pub use linux::*;

#[cfg(all(feature = "std", target_os = "windows"))]
mod windows;

#[cfg(all(feature = "std", target_os = "windows"))]
pub use windows::*;

#[cfg(all(feature = "std", target_os = "macos"))]
mod macos;

#[cfg(all(feature = "std", target_os = "macos"))]
pub use macos::*;

/// Fallback to the atomic lock strategy when the current OS is not natively
/// supported by Resync's `std` feature.
#[cfg(all(
    feature = "std",
    not(any(target_os = "linux", target_os = "windows", target_os = "macos",))
))]
pub type Os = Atomic;

#[cfg(all(feature = "std", unix))]
mod fs;

#[cfg(all(feature = "std", unix))]
pub use fs::*;
