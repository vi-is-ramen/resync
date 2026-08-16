//! Policies and traits for synchronisation primitives.
//!
//! This module provides three core traits that separate the concerns of
//! **acquisition**, **waiting**, and **shared access**:
//!
//! - [`LockPolicy`] – defines exclusive (writer) lock operations.
//! - [`SharingPolicy`] – extends [`LockPolicy`] with shared (reader)
//!   operations.
//! - [`RetryPolicy`] – defines the waiting strategy used when a lock is
//!   unavailable.
//! - [`NewLocked`] – an optional extension trait for locks that can be
//!   initialized in the acquired state.
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
//! ```
//!
//! A typical lock implementation (e.g., `Mutex<L, R>`) will hold:
//! - A policy `L: LockPolicy` for the actual lock state.
//! - A policy `R: RetryPolicy` for the waiting strategy.
//!
//! The lock's `lock()` method repeatedly calls `L::try_lock()` and, if it
//! fails, calls `R::retry()` until either the lock is acquired or the retry
//! policy aborts.
//!
//! # Safety
//!
//! [`LockPolicy`] and [`SharingPolicy`] are `unsafe` traits because they
//! directly manipulate memory and synchronisation primitives. Implementors must
//! uphold strict ordering guarantees to prevent data races. In contrast,
//! [`RetryPolicy`] and [`NewLocked`] are safe because they only affect the
//! current thread's execution and do not touch the lock's internals during
//! regular operations.
//!
//! # Examples
//!
//! See the documentation of each trait for concrete usage.

mod lock_policy;
mod new_locked;
mod retry_policy;
mod sharing_policy;
pub use lock_policy::LockPolicy;
pub use new_locked::NewLocked;
pub use retry_policy::RetryPolicy;
pub use sharing_policy::SharingPolicy;
