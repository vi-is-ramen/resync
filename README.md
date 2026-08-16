# Resync
[![Crates.io](https://img.shields.io/crates/v/resync.svg)](https://crates.io/crates/resync)
[![Documentation](https://docs.rs/resync/badge.svg)](https://docs.rs/resync)
[![License](https://img.shields.io/crates/l/resync.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-stable%20|%20beta%20|%20nightly-orange.svg)](https://www.rust-lang.org)

> **HELP WANTED**
>
> Every day, I conduct research and develop prototypes for libraries, utilities, and other developer
> tools — investing a great deal of time without any financial return. All my projects are driven purely
> by enthusiasm and willpower. I need help developing the ecosystem — specifically the Resync crate —
> and would be very grateful for issues, patches, spreading the word, or any other form of contribution.

> **ATTENTION**
>
> Resync's types are not compatible with `lock_api`'s traits. The mental models and architectures of
> Resync and `lock_api` are fundamentally incompatible. Until Rust's `const_traits` and
> `const_trait_impl` are stabilized and `lock_api` adopts a compatible model, we are not compatible.

> **GUIDEBOOK**
>
> For a comprehensive, interactive guide on the library's philosophy, design
> decisions, and advanced usage patterns, please visit the **[Resync Book](https://vi-is-ramen.github.io/resync/)**.

A LEGO-like library of synchronization primitives for Rust.

Resync provides composable building blocks for implementing locks and spin loops. Instead of a
one-size-fits-all mutex, Resync allows you to mix and match lock acquisition strategies and retry
backends at compile time using generic traits.

## Features

- **Composable Primitives**: Decouple lock acquisition (`LockPolicy`) from retry/waiting strategies (`RetryPolicy`).
- **Advanced Synchronization**: Includes `Gate` (controllable barriers), `Semaphore` (resource pooling), and `Condvar` (condition variables).
- **Lock Poisoning**: Automatically detects panics inside critical sections (when `std` is enabled) and marks locks as poisoned, protecting data integrity with granular error types (`AcquireError`, `TryLockError`).
- **`no_std` Support**: Fully compatible with `#![no_std]` environments by disabling the default `std` feature.
- **Nightly Rust Optimizations**: Automatically leverages nightly features like `const_trait_impl` and `const_default` for zero‑cost abstractions when compiled on a nightly toolchain.
- **Deadlock Prevention**: Includes composite locks (like `Nested`) that enforce a fixed acquisition and release order.
- **Granular Error Handling**: Distinct result types allow your code to differentiate between a busy lock, a successful acquisition, a timeout, and unrecoverable system aborts.

## Installation

Add Resync to your dependencies:

```shell
cargo add resync
```

To use in a `#![no_std]` environment (e.g., embedded systems or kernels), disable the default features:

```toml
[dependencies]
resync = { version = "...", default-features = false }
```

## Usage

### Basic Mutex

The `Mutex` primitive is generic over the data it protects (`T`), the lock implementation (`L`), and
the retry strategy (`R`). By default, it uses an `Atomic` lock and an OS‑yielding retry policy
(`Yield`), or a busy‑wait (`Busy`) in `no_std`.

```rust
use resync::Mutex;

fn main() {
    let mutex = Mutex::<u32>::new(42);
    {
        // Acquire the lock.
        // Returns an error if the underlying lock or retry policy reports an unrecoverable issue,
        // or if the lock was poisoned by a panicking thread.
        let mut guard = mutex.lock().unwrap();
        *guard += 1;
        assert_eq!(*guard, 43);
    } // Guard is dropped, the lock is automatically released.
}
```

### Controlling Thread Flow with `Gate`

A `Gate` acts as a controllable barrier. By default, it starts in the **closed** state, blocking any threads that call `wait()`. This is perfect for initializing a pool of workers that must not start processing until the setup phase is complete.

```rust
use resync::{Gate, lock::Atomic, retry::Yield};
use std::sync::Arc;
use std::thread;

let gate = Arc::new(Gate::<Atomic, Yield>::new()); // Starts closed

let workers: Vec<_> = (0..4).map(|i| {
    let g = Arc::clone(&gate);
    thread::spawn(move || {
        g.wait().unwrap(); // Blocks here
        println!("Worker {i} started!");
    })
}).collect();

// ... perform setup ...
gate.open(); // Unblocks all workers simultaneously

for w in workers { w.join().unwrap(); }
```

### Customizing Lock and Retry Strategies

You can swap out the underlying lock and retry implementations at compile time. For example,
you can force the mutex to use a busy‑wait spin loop instead of yielding to the OS thread scheduler.

```rust
use resync::Mutex;
use resync::lock::Atomic;
use resync::retry::Busy;

// Explicitly define the lock and retry types
let mutex: Mutex<u32, Atomic, Busy> = Mutex::new(0);
let guard = mutex.lock().unwrap();
```

## Feature Flags

- **`std`** *(enabled by default)*: Enables OS‑based retry (`retry::Yield`), OS-specific lock backends, `Condvar`, and **Lock Poisoning**. When disabled, the crate becomes `#![no_std]` and the default retry strategy falls back to `retry::Busy` (which issues `core::hint::spin_loop()`). Poisoning overhead is completely eliminated.
- **`dev`** *(disabled by default)*: Enables internal and unstable API public. It includes internal types and traits and API which is not stabilized yet (like undone or untested primitives and so on).
- **`fake`** *(disabled by default)*: Enables `Fake` type which implements `LockPolicy`, `SharingPolicy`, `NewLocked` and `RetryPolicy` but doesn't do anything. May be useful for mocks but **DANGEROUS** for use as real traits implementation.
- **`__lint`** *(disabled by default)*: Development-only feature, **MUST NOT** be enabled on build or when used as a dependency. This feature exists only for tuning `rust-analyzer` for Resync developers.


## License

Licensed under either of

* [Apache License, Version 2.0](https://github.com/vi-is-ramen/resync/blob/main/LICENSE-APACHE)
* [MIT license](https://github.com/vi-is-ramen/resync/blob/main/LICENSE-MIT)

at your option.
