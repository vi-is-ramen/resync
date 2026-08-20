# Introduction

Welcome to the **Resync Developer's Guide**! Whether you're here to fix a tricky
bug, add a new synchronization primitive, or simply understand how Resync works
under the hood, this book is written for you.

## What is Resync?

Resync is a **LEGO-like library of synchronization primitives for Rust**.
Instead of providing a single one-size-fits-all mutex, Resync allows you to mix
and match lock acquisition strategies and retry backends at compile time using
generic traits. Think of it as a box of composable building blocks for
implementing locks and spin loops that perfectly fit your use case.

Resync is designed to be:

- **Modular**: Decouple `LockPolicy` from `RetryPolicy` and combine them freely.
- **`no_std`-friendly**: Fully compatible with embedded and bare-metal
environments.
- **Zero-cost**: Leverages nightly Rust features like `const_trait_impl` for
compile-time optimizations.
- **Safe**: Offers lock poisoning, deadlock prevention via `Nested` compositors,
and granular error types.
- **Batteries-included**: Ships with production-ready `Mutex`, `Semaphore`,
`Condvar`, `Gate`, `Barrier`, and many more primitives out of the box.

```rust
// A quick taste of Resync's composable design:
use resync::{Mutex, lock::Atomic, retry::Busy};

type MySpinMutex<T> = Mutex<T, Atomic, Busy>;

let m: MySpinMutex<i32> = Mutex::new(42);
let guard = m.lock().unwrap();
assert_eq!(*guard, 42);
```

## Who is this Guide For?

This guide is written primarily for **contributors and maintainers** of Resync.
It is **not** an end-user manual — for API reference and usage examples, please
see the [docs.rs documentation](https://docs.rs/resync).

Here you will learn:

- **Project Overview** — How Resync is structured and the design philosophy
behind it.
- **Contributing Without a Deep Dive** — How to submit meaningful patches
without reading the entire codebase.
- **Feature Lifecycle** — How a new synchronization primitive goes from idea to
stable API.
- **Security Patch Lifecycle** — The sechole elision process for handling
security issues.
- **Regression Prevention** — How to stay safe from API breaks and how Resync
uses character-driven tests.

## Conventions

Throughout this guide, you'll see several types of callouts:

> **TIP**: A helpful hint for contributors.
>
> **WARNING**: Something that can easily break or cause regressions.
>
> **ATTENTION**: Something that you really need to know.
>
> **SECURITY**: Information critical to the security patch lifecycle.

Code examples are written in Rust and are fully executable via the mdBook
playground (click the play icon in the top-right corner of any code block!).

---

Ready to dive in? Let's start with [Resync: Dev's POV](./project.md) to
understand how the project is organized from a contributor's perspective.
