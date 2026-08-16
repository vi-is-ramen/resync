# 2. Core Traits

## `LockPolicy`: The Acquisition Strategy
The `LockPolicy` trait defines the raw mechanics of claiming a resource.
It is intentionally minimal and strictly atomic. It includes an associated
`Meta` type, allowing implementations to pass state (like a ticket number or
guard token) from the acquisition step (`try_lock`) to the release step
(`free`).

## `RetryPolicy`: The Waiting Strategy
The `RetryPolicy` trait defines what the CPU should do when a `LockPolicy`
reports contention. This could be a tight CPU pause loop (`retry::Busy`),
yielding to the OS scheduler (`retry::Yield`), or even a custom exponential
backoff strategy.

## `SharingPolicy`: The Read-Write Semantics
Extends `LockPolicy` to support shared (reader) access. This allows a single
lock primitive to support both reader and writer access, forming the basis for
the `Sharex` (RwLock) primitive.

## `NewLocked`: The Initialization Strategy
Allows locks to be created in an already-acquired (locked) state. This is
crucial for primitives like `Gate` that must start closed to prevent
Time-of-Check to Time-of-Use (TOCTOU) races. By segregating this into a
separate trait, Resync follows the Interface Segregation Principle.

## `Mutex` & `Sharex`: The Composition
These structs bind a `LockPolicy`, a `RetryPolicy`, and the protected data (`T`)
together, providing a safe, RAII-based interface (`ExGuard` and `ShGuard`).
They also manage the **Lock Poisoning** state, automatically detecting thread
panics (when `std` is enabled) to protect data integrity.
