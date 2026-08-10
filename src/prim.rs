//! Foundational synchronization primitives built on top of locks and spins.

mod mutex;
mod rwlock;

pub use mutex::*;
pub use rwlock::*;

// TODO: more synchronization primitives
