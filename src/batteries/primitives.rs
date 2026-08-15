//! High-level synchronization primitives built on top of low-level policies.
//!
//! This module contains the user-facing synchronization primitives.

mod mutex;

pub use mutex::*;
