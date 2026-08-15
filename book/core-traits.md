# 2. Core Traits

## `LockPolicy`: The Acquisition Strategy

The `LockPolicy` trait defines the raw mechanics of claiming a resource.
It is intentionally minimal and strictly atomic.

## `RetryPolicy`: The Waiting Strategy

The `RetryPolicy` trait defines what the CPU should do when a
`LockPolicy` reports contention. This could be a tight CPU pause loop
(`retry::Busy`) or yielding to the OS scheduler
(`retry::Yield`).

## `Mutex`: The Composition

The `Mutex` struct binds a `LockPolicy`, a `RetryPolicy`,
and the protected data (`T`) together, providing a safe, RAII-based
interface (`ExGuard`).

## Also

### `SharingPolicy`: The Sharing

The `SharingPolicy` trait defines the raw mechanics of mutable and immutable claiming of resources. It can be used for implementing custom `RwLock` backends (like `Sharex`).
