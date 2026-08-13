macro_rules! x {
    ($a:literal $b:ident) => {
        #[cfg(target_os = $a)]
        mod $b;
        #[cfg(target_os = $a)]
        pub use $b::*;
    };
}

x!("linux" linux);

/// Fallback atomic implementation as this platform isn't supported by Resync.
/// Same as [`crate::lock::Atomic`].
#[cfg(not(any(target_os = "linux")))]
pub type Os = crate::lock::Atomic;
