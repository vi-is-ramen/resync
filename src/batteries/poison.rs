//! Built-in implementations of [`PoisonPolicy`](crate::traits::PoisonPolicy).

mod nop;
pub use nop::*;

#[cfg(any(std, docsrs))]
mod stdp;
#[cfg(any(std, docsrs))]
pub use stdp::*;

#[cfg(any(docsrs, feature = "fake"))]
pub use super::fake::*;

/// The default poison policy.
///
/// As `std` feature enabled, it is `StdPoison`.
#[cfg(all(std, not(docsrs)))]
pub type DefaultPoison = StdPoison;

/// The default poison policy.
///
/// As `std` feature disabled, it is `NoPoison`.
#[cfg(all(no_std, not(docsrs)))]
pub type DefaultPoison = NoPoison;

/// The default poison policy.
#[cfg(docsrs)]
pub type DefaultPoison = Fake;
