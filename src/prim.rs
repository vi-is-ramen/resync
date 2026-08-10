//! Foundational synchronization primitives built on top of locks and spins.

mod mutex;

pub use mutex::*;

// TODO: more synchronization primitives
