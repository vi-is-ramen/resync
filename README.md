# Resync

[![Crates.io](https://img.shields.io/crates/v/resync.svg)](https://crates.io/crates/resync)
[![Documentation](https://docs.rs/resync/badge.svg)](https://docs.rs/resync)
[![License](https://img.shields.io/crates/l/resync.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-stable%20|%20beta%20|%20nightly-orange.svg)](https://www.rust-lang.org)

> **HELP WANTED**
>
> Every day, I conduct research and develop prototypes for libraries, utilities,
> and other developer tools — investing a great deal of time without any
> financial return. All my projects are driven purely by enthusiasm and
> willpower. I need help developing the ecosystem — specifically the Resync
> crate - and would be very grateful for issues, patches, spreading the word, or
> any other form of contribution.

> **ATTENTION**
>
> Resync's types are not compatible with `lock_api`'s traits. The mental models
> and architectures of Resync and `lock_api` are fundamentally incompatible.
> Until Rust's `const_traits` and `const_trait_impl` are stabilized and
> `lock_api` adopts a compatible model, we are not compatible.

> **GUIDEBOOK**
>
> For a comprehensive, interactive guide on the library's philosophy, design
> decisions, and advanced usage patterns, please visit the
> **[Resync Book](https://vi-is-ramen.github.io/resync/)**.

A LEGO-like library of synchronization primitives for Rust.

Resync provides composable building blocks for implementing locks and spin loops.
Instead of a one-size-fits-all mutex, Resync allows you to mix and match lock
acquisition strategies and retry backends at compile time using generic traits.

## Features

- **Composable Primitives**: Decouple lock acquisition (`LockPolicy`) from retry
/waiting strategies (`RetryPolicy`).
- **Advanced Synchronization**: Includes `Gate` (controllable barriers),
`Semaphore` (resource pooling), and `Condvar` (condition variables).
- **Lock Poisoning**: Automatically detects panics inside critical sections
(when `std` is enabled) and marks locks as poisoned, protecting data integrity
with granular error types (`AcquireError`, `TryLockError`).
- **`no_std` Support**: Fully compatible with `#![no_std]` environments by
disabling the default `std` feature.
- **Nightly Rust Optimizations**: Automatically leverages nightly features like
`const_trait_impl` and `const_default` for zero‑cost abstractions when compiled
on a nightly toolchain.
- **Deadlock Prevention**: Includes composite locks (like `Nested`) that enforce
a fixed acquisition and release order.
- **Granular Error Handling**: Distinct result types allow your code to
differentiate between a busy lock, a successful acquisition, a timeout, and
unrecoverable system aborts.

## Batteries Included

While Resync is built on modularity, it ships with an impressive,
production-ready arsenal of synchronization primitives and backend policies out
of the box:

| Category | Batteries | Description |
| :--- | :--- | :--- |
| **Primitives** | `Mutex`, `Sharex` | Standard exclusive and read-write locks with poisoning support. |
| **Flow Control** | `Gate`, `Barrier`, `Condvar`, `Semaphore` | Controllable barriers, event waiting, and resource pooling. |
| **Lock Backends** | `Atomic`, `Os`, `Fs`, `Irq` | Pure spinlocks, OS futexes/SRW, file locks, and IRQ-disabling locks. |
| **Compositors** | `Nested`, `Shield` | Deadlock prevention via strict ordering, and writer-fairness wrappers. |
| **Retry Strategies** | `Busy`, `Yield` | CPU pause instructions (`spin_loop`) or OS thread yielding. |

## How does it compare?

| Feature | `std::sync` | `parking_lot` | `spin` | **`resync`** |
| :--- | :---: | :---: | :---: | :---: |
| **Composable Backends** | - | - | - | + |
| **`#![no_std]` Native** | - | * (requires features) | + | + |
| **Granular Errors** | * (Poison only) | - (Panics/Infallible) | - | + (`Lock`, `Retry`, `Poison`) |
| **Adaptive Retry Policies** | - | - | - | + |
| **Writer-Fairness (`Shield`)** | - | - | - | + |
| **TOCTOU-free Initialization** | - | - | - | + (`NewLocked` trait) |

## Installation

Add Resync to your dependencies:

```shell
cargo add resync
```

To use in a `#![no_std]` environment (e.g., embedded systems or kernels),
disable the default features:

```toml
[dependencies]
resync = { version = "...", default-features = false }
```

## Usage

### Basic Mutex

```rust
use resync::Mutex;

fn main() {
    let mutex = Mutex::<u32>::new(42);
    {
        let mut guard = mutex.lock().unwrap();
        *guard += 1;
        assert_eq!(*guard, 43);
    }
}
```

### Controlling Thread Flow with `Gate`

A `Gate` acts as a controllable barrier. By default, it starts in the **closed**
state, blocking any threads that call `wait()`.

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

## Feature Flags

- **`std`** *(enabled by default)*: Enables OS‑based retry (`retry::Yield`),
OS-specific lock backends, `Condvar`, and **Lock Poisoning**.
- **`dev`** *(disabled by default)*: Enables internal and unstable API public.
- **`fake`** *(disabled by default)*: Enables `Fake` type for mocks.
- **`__lint`** *(disabled by default)*: Development-only feature for
`rust-analyzer`.

## License

Licensed under either of

* [Apache License, Version 2.0](https://github.com/vi-is-ramen/resync/blob/main/LICENSE-APACHE)
* [MIT license](https://github.com/vi-is-ramen/resync/blob/main/LICENSE-MIT)

at your option.
