//! Built-in implementations of [`LockPolicy`](crate::traits::LockPolicy) and
//! [`SharingPolicy`](crate::traits::SharingPolicy).
//!
//! This module provides several ready-to-use lock backends that can be plugged
//! into higher-level primitives like [`Mutex`](crate::Mutex). The available
//! implementations depend on the target operating system and enabled features.

macro_rules! inc {
    ($( $(#[$($m:meta),*])* $x:ident),* $(,)?) => {
        $(
            $(#[$($m),*])*
            pub(crate) mod $x;
            $(#[$($m),*])*
            pub use $x::*;
        )*
    };
}

inc! {
    // portable, well-known locks
    atomic,
    nested,
    shield,

    // mocks only
    #[cfg(any(docsrs, feature = "fake"))]
    fake,

    // experimental (so `dev`)
    #[cfg(all(unix, std, dev))]
    fs,

    // experimental
    #[cfg(dev)]
    irq,

    // OS-dependent
    #[cfg(all(std, target_os = "linux"))]
    linux,

    // OS-dependent
    #[cfg(all(std, target_os = "windows"))]
    windows,

    // OS-dependent
    #[cfg(all(std, target_os = "macos"))]
    macos,
}

/// Default lock implementation for current environment.
///
/// As Linux target selected, it is Futex.
#[cfg(all(std, target_os = "linux", not(docsrs)))]
pub type DefaultLock = Futex;

/// Default lock implementation for current environment.
///
/// As Windows target selected, it is SRW.
#[cfg(all(std, target_os = "windows", not(docsrs)))]
pub type DefaultLock = Srw;

/// Default lock implementation for current environment.
///
/// As macOS target selected, it is Rwl.
#[cfg(all(std, target_os = "macos", not(docsrs)))]
pub type DefaultLock = Rwl;

/// Default lock implementation for current environment.
///
/// As bare-metal target selected, it is Atomic.
#[cfg(any(no_std, not(docsrs)))]
pub type DefaultLock = Atomic;

/// Default lock implementation for current environment.
///
/// It becomes futex on Linux, SRW on Windows and rwlock_t on macOS.
/// If used in bare-metal environments (no_std), it becomes Atomic.
#[cfg(docsrs)]
pub type DefaultLock = Fake;
