//! .

mod os;
mod stub;

pub use os::*;
pub use stub::*;

/// Default park strategy for current environment,
/// selected by Resync. Good option if you just
/// writing something platform-aware without
/// deep-minding about synchronization.
#[cfg(feature = "std")]
pub type DefaultPark = Os;

/// Default park strategy for current environment,
/// selected by Resync. Good option if you just
/// writing something platform-aware without
/// deep-minding about synchronization.
#[cfg(not(feature = "std"))]
pub type DefaultPark = Stub;

/// # Safety
pub unsafe trait IPark: core::default::Default
{
    /// .
    fn park(&self);

    /// .
    fn free(&self);
}
