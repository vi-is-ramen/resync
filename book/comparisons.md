# 4. Comparisons with the Ecosystem

How does Resync stack up against the standard library and popular third-party
crates?

## `std::sync` vs `resync`

The standard library provides `Mutex` and `RwLock`, but they are monolithic.
You cannot change how they wait (they always yield/park via the OS), and they
are strictly tied to `std`.  Furthermore, `std::sync::Mutex::lock()` returns a
`PoisonError`, which conflates poisoning with other potential lock failures.
Resync separates these concerns using `AcquireError`, distinguishing between
**Poisoned** (data inconsistency), **Lock** (fatal OS/hardware error), and
**Retry** (timeout).

## `parking_lot` vs `resync`

`parking_lot` is incredibly fast and widely used. However, it hardcodes its
OS-level parking mechanism. If you are writing a hybrid application where 99%
of locks should park in the OS, but 1% of locks (e.g., inside a specific audio
processing callback or embedded context) must *never* yield to the OS,
`parking_lot` forces you to use a completely different crate (like `spin`) for
that 1%. Resync allows you to use the exact same `Mutex<T>` API, simply
swapping the generic parameter from `Os` to `Atomic` and `Yield` to `Busy` for
that specific critical section.

## `spin` vs `resync`

The `spin` crate provides excellent pure spinlocks, but lacks advanced
composable features like `SharingPolicy` (RW semantics), `NewLocked`
(TOCTOU-free initialization), and adaptive retry strategies out of the box.
Resync's `Atomic` backend provides similar raw performance, but plugs into a
much richer ecosystem of high-level primitives like `Gate` and `Semaphore`.

## `lock_api` vs `resync::api`

`lock_api` is the standard for abstracting over locks in the ecosystem. However,
its `RawMutex` trait assumes that locking is *infallible* (it cannot return an
error, nor can it represent timeouts or poisoning). Resync provides its own
`resync::api::Mutex` trait, which embraces granular error handling. It allows
generic code to accept *any* compatible synchronization primitive while
preserving safety guarantees like poisoning and timeout handling.
