//! Built-in implementations of [`RetryPolicy`](crate::traits::RetryPolicy).
//!
//! This module provides ready-to-use retry strategies that can be plugged
//! into higher-level primitives like [`Mutex`](crate::Mutex). The retry policy
//! determines what the CPU should do when a lock acquisition attempt fails
//! due to contention.

mod busy;
mod r#yield;

#[cfg(feature = "fake")]
pub use super::lock::{Fake, FakeError};
pub use busy::*;
pub use r#yield::*;
