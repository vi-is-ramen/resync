//! A LEGO-like synchronization primitives library.
//!
//! This crate provides building blocks for implementing locks and spin loops,
//! with composable traits and backends that can be swapped at compile time.
//!
//! Guidebook: [`guide`]
//!
//! # Features
//! - `std` (enabled by default): enables OS‑based spinning ([`spin::Os`]).
//! - Nightly features: const traits and const [`core::default::Default`]
//!   implementations when the `nightly` rustc channel is detected.

#![cfg_attr(nightly,
    feature(
        // nightly features
        const_trait_impl,
        const_default,
    )
)]
#![cfg_attr(all(test, nightly), feature(derive_const))]
// don't link to libstd if `std` feature disabled
#![cfg_attr(not(feature = "std"), no_std)]

pub mod lock;
mod prim;
mod result;
pub mod spin;

pub use lock::ILock;
pub use prim::*;
pub use result::*;
pub use spin::ISpin;

pub mod guide;
