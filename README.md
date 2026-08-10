# resync

[![Crates.io](https://img.shields.io/crates/v/resync.svg)](https://crates.io/crates/resync)
[![Documentation](https://docs.rs/resync/badge.svg)](https://docs.rs/resync)
[![License](https://img.shields.io/crates/l/resync.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-stable%20|%20beta%20|%20nightly-orange.svg)](https://www.rust-lang.org)

> **HELP WANTED**<br/>
> Every day, I conduct research and develop prototypes for libraries, utilities, and other developer
> tools — investing a great deal of time without any financial return. All my projects are driven purely
> by enthusiasm and willpower. I need help developing the ecosystem — specifically the `resync` crate —
> and would be very grateful for issues, patches, or any other form of contribution (such as spreading the word).

A LEGO-like synchronization primitives library for Rust.

`resync` provides composable building blocks for implementing locks and spin loops. Instead of a one-size-fits-all mutex, `resync` allows you to mix and match lock acquisition strategies and spin-wait backends at compile time using generic traits.

## Features

- **Composable Primitives**: Decouple lock acquisition (`ILock`) from spin-wait strategies (`ISpin`).
- **`no_std` Support**: Fully compatible with `#![no_std]` environments by disabling the default `std` feature.
- **Nightly Rust Optimizations**: Automatically leverages nightly features like `const_trait_impl` and `const_default` for zero-cost abstractions when compiled on a nightly toolchain.
- **Deadlock Prevention**: Includes composite locks (like `Nested`) that enforce a fixed acquisition and release order.
- **Granular Error Handling**: Distinct result types (`LockResult` and `SpinResult`) allow your code to differentiate between a busy lock, a successful acquisition, and unrecoverable system aborts.

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
the spin strategy (`S`). By default, it uses an `Atomic` lock and an OS-yielding spin
(or `Busy` spin in `no_std`).

```rust
use resync::Mutex;

fn main() {
    let mutex = Mutex::<u32>::new(42);

    {
        // Acquire the lock. 
        // Returns None if the underlying lock/spin reports an unrecoverable Abort.
        let mut guard = mutex.lock().unwrap();
        *guard += 1;
        assert_eq!(*guard, 43);
    } // Guard is dropped, lock is automatically released
}
```

### Customizing Lock and Spin Strategies
You can swap out the underlying lock and spin implementations at compile time. For example,
you can force the mutex to use a busy-wait spin loop instead of yielding to the OS thread scheduler.

```rust
use resync::Mutex;
use resync::lock::Atomic;
use resync::spin::Busy;

// Explicitly define the lock and spin types
let mutex: Mutex<u32, Atomic, Busy> = Mutex::new(0);
let guard = mutex.lock().unwrap();
```

### Using Locks Directly (`ILock`)
If you don't need a full `Mutex` and just need raw lock semantics, you can use the `ILock` implementations directly.

```rust
use resync::{ILock, LockResult};
use resync::lock::Atomic;

let lock = Atomic::new();

match lock.try_lock() {
    LockResult::Done  => println!("Successfully acquired!"),
    LockResult::Fail  => println!("Lock is currently held by someone else."),
    LockResult::Abort => println!("Unrecoverable system error occurred."),
}

// Release the lock (idempotent)
lock.free();
```

### Composite Locks (Deadlock Prevention)
The `Nested` lock allows you to compose two locks together. It always acquires the first lock (`L1`)
before the second (`L2`), and releases them in reverse order (`L2` then `L1`). This deterministic
ordering helps prevent deadlocks when multiple locks are required.

```rust
use resync::lock::{Atomic, Nested, ILock};
use resync::LockResult;

type SafeNestedLock = Nested<Atomic, Atomic>;

let lock = SafeNestedLock::default();
if lock.try_lock() == LockResult::Done {
    println!("Acquired both inner locks safely!");
    lock.free(); // Releases L2, then L1
}
```

## Extensibility

Because `resync` relies on traits (`ILock` and `ISpin`), you can implement your own lock or
spin strategies (e.g., ticket locks, exponential backoff spins, or hardware-specific pause
instructions) and plug them directly into the `Mutex`.

```rust
use resync::{ISpin, SpinResult};

struct ExponentialBackoffSpin { /* ... */ }

impl ISpin for ExponentialBackoffSpin {
    fn spin(&self) -> SpinResult {
        // Custom backoff logic here
        SpinResult::Ok 
    }
}
```

## Feature Flags

- **`std`** *(enabled by default)*: Enables OS-based spinning (`spin::Os`), which calls `std::thread::yield_now()`. 
- **`no_std`**: If disabled, the crate becomes `#![no_std]` and the default spin strategy falls back to `spin::Busy` (which issues `core::hint::spin_loop()`).

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

Made with ❤️ for Rust community
