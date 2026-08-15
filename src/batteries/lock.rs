//! LockPolicy & SharingPolicy batteries

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

/// Fallback to the atomic lock strategy - OS is not supported by Resync.
#[cfg(all(
    feature = "std",
    not(any(target_os = "linux", target_os = "windows", target_os = "macos",))
))]
pub type Os = Atomic;
