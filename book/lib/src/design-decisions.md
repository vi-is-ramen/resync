# 3. Design Decisions

## Why no `is_locked()` or `is_free()` in `LockPolicy`?
A common question when designing lock APIs is:
_"Why can't I check if the lock is currently held before trying to acquire it?"_

In concurrent programming, checking a state and then acting on it introduces a
**Time-of-Check to Time-of-Use (TOCTOU)** race condition. Consider this
hypothetical anti-pattern:

```rust
// HYPOTHETICAL BAD CODE
if !lock.is_locked() {
    // Another thread could acquire the lock RIGHT HERE
    lock.lock();
}
```

By the time you call lock() after checking is_locked(), the state may
have already changed. The check is not only wasted CPU cycles, but it gives
a false sense of security and predictability.

By omitting state-querying methods, Resync forces you to use
`LockPolicy::try_lock`, which is an atomic check-and-acquire
operation. The result of the operation tells you the state at the exact
moment the atomic instruction executed, eliminating TOCTOU bugs by design.

# Why `AcquireError` and `TryLockError` instead of just `PoisonError`?
Standard library locks return `PoisonError<Guard>`, which conflates poisoning
with other potential lock failures. Resync separates these concerns using
`AcquireError` and `TryLockError`. These enums distinguish between:
- **Poisoned**: A previous thread panicked, and the data might be inconsistent.
- **Lock**: A fatal, unrecoverable error occurred in the underlying `LockPolicy`
(e.g., OS resource exhaustion).
- **Retry**: The `RetryPolicy` aborted the wait loop (e.g., due to a timeout).

This granular error handling allows `no_std` environments and complex systems to
react appropriately to timeouts or hardware failures without relying on panics.

# Why is Lock Poisoning `std`-only?
Lock poisoning relies on detecting whether the current thread is unwinding due to
a panic (`std::thread::panicking()`). In `#![no_std]` environments (like kernels or
embedded systems), a panic typically triggers an immediate abort or system reset,
making the concept of "recovering from a panic inside a lock" inapplicable. Therefore,
the poisoning machinery (`AtomicBool` flags and guard checks) is conditionally compiled
only when the `std` feature is enabled, ensuring zero overhead for bare-metal targets.

# Why is `LockPolicy::free` taking metadata?
The `free` method requires the `Meta` object returned by `try_lock`. For simple locks
(like `Atomic`), `Meta` is just `()`, making the release trivial. However, for more
complex locks (like ticket locks or OS futexes that need to track waiter queues), this
metadata is essential to correctly release the exact lock instance or wake the correct
threads.

Because the `Meta` is tied to the specific acquisition, it also prevents accidentally
releasing a lock you don't own or releasing it multiple times with stale state. For
simple locks where `Meta = ()`, calling `free(&())` on an already free lock remains a
guaranteed no-op, which dramatically simplifies the implementation of composite locks
(like `lock::Nested`) and error-handling paths. If an abort occurs halfway through
acquiring a nested lock, the cleanup code can safely call `free` on all inner locks
that successfully returned their metadata.

# Why `lock::Nested`?
Deadlocks often occur when multiple locks are acquired in inconsistent orders across
different threads. `lock::Nested` enforces a strict, deterministic acquisition order
(`L1` then `L2`) and a reverse release order (`L2` then `L1`). This provides a
compile-time building block for safe multi-resource locking.

## Why `AcquireError` instead of just `PoisonError`?
Standard library locks return `PoisonError<Guard>`, which conflates poisoning
with other potential lock failures. Resync separates these concerns using
`AcquireError` and `TryLockError`. These enums distinguish between:
- **Poisoned**: A previous thread panicked, and the data might be inconsistent.
- **Lock**: A fatal, unrecoverable error occurred in the underlying `LockPolicy`
(e.g., OS resource exhaustion).
- **Retry**: The `RetryPolicy` aborted the acquisition loop (e.g., due to a
timeout).

This granular error handling allows `no_std` environments and complex systems
to react appropriately to timeouts or hardware failures without relying on
panics.

## Why `Shield` instead of a custom RwLock?
Writer starvation is a common problem in RwLocks. Instead of hardcoding a
"writer-preference" mode into the base `Os` or `Atomic` locks (which adds
overhead to readers who don't need it), Resync provides `Shield`. It acts as a
transparent wrapper that intercepts `try_lock` and `try_share`, dynamically
blocking readers only when a writer is actively waiting. This keeps the base
locks fast and simple while providing a composable solution for fairness.
