# 3. Design Decisions

## Why no `is_locked()` or `is_free()` in `LockPolicy`?

A common question when designing lock APIs is: _"Why can't I check if the
lock is currently held before trying to acquire it?"_

In concurrent programming, checking a state and then acting on it introduces
a **Time-of-Check to Time-of-Use (TOCTOU)** race condition. Consider this
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

# Why LockResult and RetryResult instead of bool?

Standard library locks often return bool or Result<T, PoisonError>.
Resync uses explicit enums (`LockStatus<M>` inside `LockResult<M, E>` and
`RetryResult<E>`) to handle distinct states without relying on
panics:

- `Done(M)` / `Ok(())`: Success. The lock is acquired, and any necessary
metadata is returned to be passed back during release.
- `Fail`: Contention. The lock is held, but the system is healthy. The
caller should spin or yield.
- `Err(E)`: Unrecoverable error. The lock is "poisoned", the underlying
hardware failed, or the retry strategy detected a timeout. This allows
`no_std` environments to handle catastrophic failures gracefully without
unwinding panics.

# Why is `LockPolicy::free` taking metadata?

The `free` method requires the `Meta` object returned by `try_lock`. For
simple locks (like `Atomic`), `Meta` is just `()`, making the release
trivial. However, for more complex locks (like ticket locks or OS futexes
that need to track waiter queues), this metadata is essential to correctly
release the exact lock instance or wake the correct threads.

Because the `Meta` is tied to the specific acquisition, it also prevents
accidentally releasing a lock you don't own or releasing it multiple times
with stale state. For simple locks where `Meta = ()`, calling `free(&())`
on an already free lock remains a guaranteed no-op, which dramatically
simplifies the implementation of composite locks (like `lock::Nested`)
and error-handling paths. If an abort occurs halfway through acquiring a
nested lock, the cleanup code can safely call `free` on all inner locks
that successfully returned their metadata.

# Why `lock::Nested`?

Deadlocks often occur when multiple locks are acquired in inconsistent
orders across different threads. lock::Nested enforces a strict,
deterministic acquisition order (L1 then L2) and a reverse release
order (L2 then L1). This provides a compile-time building block for
safe multi-resource locking.
