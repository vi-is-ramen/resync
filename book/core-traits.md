# 2. Core Traits

## `LockPolicy`: The Acquisition Strategy
The `LockPolicy` trait defines the raw mechanics of claiming a resource.
It is intentionally minimal and strictly atomic. It includes an associated `Meta` type,
allowing implementations to pass state (like a ticket number or guard token) from the
acquisition step (`try_lock`) to the release step (`free`).

## `RetryPolicy`: The Waiting Strategy
The `RetryPolicy` trait defines what the CPU should do when a `LockPolicy` reports
contention. This could be a tight CPU pause loop (`retry::Busy`) or yielding to the
OS scheduler (`retry::Yield`).

## `NewLocked`: The Initialization Strategy
The `NewLocked` trait extends `LockPolicy` to allow locks to be created in an
already-acquired (locked) state. This is crucial for primitives like `Gate` that must
start closed to prevent Time-of-Check to Time-of-Use (TOCTOU) races. By segregating this
into a separate trait, Resync follows the Interface Segregation Principle: basic locks
don't need to implement locked initialization if it doesn't make sense for their backend.

## `Mutex`: The Composition
The `Mutex` struct binds a `LockPolicy`, a `RetryPolicy`, and the protected data (`T`)
together, providing a safe, RAII-based interface (`ExGuard`).

## Also
### `SharingPolicy`: The Sharing
The `SharingPolicy` trait defines the raw mechanics of mutable and immutable claiming of
resources. It can be used for implementing custom `RwLock` backends (like `Sharex`).
