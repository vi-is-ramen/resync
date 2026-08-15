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
Resync uses explicit enums (LockStatus inside LockResult and
RetryResult) to handle distinct states without relying on
panics:

- Done / Ok: Success.
- Fail: Contention. The lock is held, but the system is healthy. The
caller should spin or yield.
- Err: Unrecoverable error. The lock is "poisoned", the underlying
hardware failed, or the retry strategy detected a timeout. This allows
`no_std` environments to handle catastrophic failures gracefully without
unwinding panics.

# Why is `LockPolicy::free` idempotent?

Calling free() on an already free lock is a guaranteed no-op. This
dramatically simplifies the implementation of composite locks (like
lock::Nested) and error-handling paths. If an abort occurs
halfway through acquiring a nested lock, the cleanup code can safely call
free() on all inner locks without needing to track which specific ones
were successfully acquired.

# Why `lock::Nested`?

Deadlocks often occur when multiple locks are acquired in inconsistent
orders across different threads. lock::Nested enforces a strict,
deterministic acquisition order (L1 then L2) and a reverse release
order (L2 then L1). This provides a compile-time building block for
safe multi-resource locking.
