//! Policies and traits for synchronisation primitives.
//!
//! This module provides core traits that separate the concerns of
//! **acquisition**, **waiting**, **shared access**, and **poisoning**:
//!
//! - [`LockPolicy`] – defines exclusive (writer) lock operations.
//! - [`SharingPolicy`] – extends [`LockPolicy`] with shared (reader)
//!   operations.
//! - [`RetryPolicy`] – defines the waiting strategy used when a lock is
//!   unavailable.
//! - [`NewLocked`] – an optional extension trait for locks that can be
//!   initialized in the acquired state.
//! - [`PoisonPolicy`] – defines how a lock reacts to thread panics.
//!
//! # Design Philosophy
//!
//! Instead of hard‑coding a specific mutex or spinlock implementation, these
//! traits allow the construction of generic locks that are parameterised by
//! their underlying behaviour. This enables:
//!
//! - **Performance tuning**: Swap a spin‑based policy for a parking‑based one
//!   without changing the lock's core logic.
//! - **Testing**: Inject mock policies to simulate lock contention or errors.
//! - **Portability**: Use the same lock interface on bare‑metal (where parking
//!   may not be available) and on hosted OSes.
//! - **Zero-cost abstractions**: Disable poisoning overhead entirely via
//!   [`PoisonPolicy`] for critical paths.
//!
//! # Relationship Between Traits
//!
//! ```text
//! LockPolicy (exclusive ops)
//!     ^           ^
//! SharingPolicy   NewLocked (locked initialization)
//! (adds shared ops)
//!
//! RetryPolicy (used by both during waiting)
//! PoisonPolicy (used by guards on drop)
//! ```
//!
//! # Safety
//!
//! [`LockPolicy`] and [`SharingPolicy`] are `unsafe` traits because they
//! directly manipulate memory and synchronisation primitives. Implementors must
//! uphold strict ordering guarantees to prevent data races. In contrast,
//! [`RetryPolicy`], [`NewLocked`], and [`PoisonPolicy`] are safe because they
//! only affect the current thread's execution or manage isolated state.
//!
//! # Examples
//!
//! See the documentation of each trait for concrete usage.
mod lock_policy;
mod new_locked;
mod poison_policy;
mod retry_policy;
mod sharing_policy;
mod thread_id;

pub use lock_policy::LockPolicy;
pub use new_locked::NewLocked;
pub use poison_policy::PoisonPolicy;
pub use retry_policy::RetryPolicy;
pub use sharing_policy::SharingPolicy;
pub use thread_id::StableThreadId;
