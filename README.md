# Resync

[![Crates.io](https://img.shields.io/crates/v/resync.svg)](https://crates.io/crates/resync)
[![Documentation](https://docs.rs/resync/badge.svg)](https://docs.rs/resync)
[![License](https://img.shields.io/crates/l/resync.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-stable%20|%20beta%20|%20nightly-orange.svg)](https://www.rust-lang.org)

> **HELP WANTED**  
> Every day, I conduct research and develop prototypes for libraries, utilities, and other developer
> tools — investing a great deal of time without any financial return. All my projects are driven purely
> by enthusiasm and willpower. I need help developing the ecosystem — specifically the Resync crate —
> and would be very grateful for issues, patches, spreading the word, or any other form of contribution.

> **ATTENTION**  
> Resync's types are not compatible with `lock_api`'s traits. The mental models and architectures of
> Resync and `lock_api` are fundamentally incompatible. Until Rust's `const_traits` and
> `const_trait_impl` are stabilized and `lock_api` adopts a compatible model, we are not compatible.

A LEGO-like library of synchronization primitives for Rust.

Resync provides composable building blocks for implementing locks and spin loops. Instead of a
one-size-fits-all mutex, Resync allows you to mix and match lock acquisition strategies and retry
backends at compile time using generic traits.

## Features

- **Composable Primitives**: Decouple lock acquisition (`LockPolicy`) from retry/waiting strategies (`RetryPolicy`).
- **`no_std` Support**: Fully compatible with `#![no_std]` environments by disabling the default `std` feature.
- **Nightly Rust Optimizations**: Automatically leverages nightly features like `const_trait_impl` and `const_default` for zero‑cost abstractions when compiled on a nightly toolchain.
- **Deadlock Prevention**: Includes composite locks (like `Nested`) that enforce a fixed acquisition and release order.
- **Granular Error Handling**: Distinct result types (`LockResult` and `RetryResult`) allow your code to differentiate between a busy lock, a successful acquisition, and unrecoverable system aborts.

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
        // Returns an error if the underlying lock or retry policy reports an unrecoverable issue.
        let mut guard = mutex.lock().unwrap();
        *guard += 1;
        assert_eq!(*guard, 43);
    } // Guard is dropped, the lock is automatically released.
}
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

### Using Locks Directly (`LockPolicy`)

If you don't need a full `Mutex` and just need raw lock semantics, you can use the `LockPolicy` implementations directly.

```rust
use resync::traits::LockPolicy;
use resync::lock::Atomic;
use resync::LockStatus;

let lock = Atomic::new();

match unsafe { lock.try_lock(0) } {
    Ok(LockStatus::Done)  => println!("Successfully acquired!"),
    Ok(LockStatus::Fail)  => println!("Lock is currently held by someone else."),
    Err(e) => println!("Unrecoverable system error: {:?}", e),
}

// Release the lock (idempotent)
unsafe { lock.free() };
```

### Composite Locks (Deadlock Prevention)

The `Nested` lock allows you to compose two locks together. It always acquires the first lock (`L1`)
before the second (`L2`), and releases them in reverse order (`L2` then `L1`). This deterministic
ordering helps prevent deadlocks when multiple locks are required.

```rust
use resync::lock::{Atomic, Nested};
use resync::traits::LockPolicy;
use resync::LockStatus;

type SafeNestedLock = Nested<Atomic, Atomic>;

let lock = SafeNestedLock::default();
if unsafe { lock.try_lock(0) } == Ok(LockStatus::Done) {
    println!("Acquired both inner locks safely!");
    unsafe { lock.free() }; // Releases L2, then L1
}
```

## Extensibility

Because Resync relies on traits (`LockPolicy` and `RetryPolicy`), you can implement your own lock or
retry strategies (e.g., ticket locks, exponential backoff, or hardware‑specific pause
instructions) and plug them directly into the `Mutex`.

```rust
use resync::traits::RetryPolicy;
use resync::RetryResult;

struct ExponentialBackoffRetry { /* ... */ }

impl RetryPolicy for ExponentialBackoffRetry {
    type Error = core::convert::Infallible;

    fn retry(&self, current_iteration: usize) -> RetryResult<Self::Error> {
        // Custom backoff logic here
        Ok(())
    }
}
```

## Feature Flags

- **`std`** *(enabled by default)*: Enables OS‑based retry (`retry::Yield`), which calls `std::thread::yield_now()`.
- **`no_std`**: When disabled, the crate becomes `#![no_std]` and the default retry strategy falls back to `retry::Busy` (which issues `core::hint::spin_loop()`).

## Minimum Supported Rust Version (MSRV)

Resync is continuously tested against the latest **stable**, **beta**, and **nightly** Rust channels.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](https://github.com/vi-is-ramen/resync/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](https://github.com/vi-is-ramen/resync/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

---

Made with ❤️ for the Rust community
