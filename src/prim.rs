//! Foundational synchronization primitives built on top of locks and spins.

mod barrier;
mod gate;
mod mutex;
mod rwlock;

pub use barrier::*;
pub use gate::*;
pub use mutex::*;
pub use rwlock::*;

// TODO: more synchronization primitives
