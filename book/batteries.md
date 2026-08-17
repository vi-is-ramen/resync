# 3. Batteries Included

While the core philosophy of Resync is modularity, it doesn't mean you have to
build everything from scratch. Resync ships with an impressive,
production-ready arsenal of synchronization primitives and backend policies
out of the box.

## High-Level Primitives

- **`Mutex<T, L, R>`**: The standard mutual exclusion lock. Protects data and
supports lock poisoning.
- **`Sharex<T, L, R>`**: A read-write lock (RwLock) allowing multiple
concurrent readers or a single exclusive writer.
- **`Gate<L, R>`**: A controllable barrier. Starts closed (via
`NewLocked`) and blocks threads until explicitly opened. Perfect for thread
pool initialization.
- **`Semaphore<L, R>`**: A counting semaphore for limiting concurrent access
to a pool of resources (e.g., DB connections).
- **`Condvar`**: A condition variable for event-based waiting, fully respecting
the poisoning semantics of the associated `Mutex`.
- **`Barrier<R>`**: A synchronization primitive that blocks a set of threads
until all of them have reached a certain point.
- **`Once<T, L, R, P>`**: A primitive for one-time lazy initialization (similar
to `std::sync::OnceLock`). It uses a fast-path atomic check and falls back to the
`LockPolicy` only during the initialization phase. Fully respects lock poisoning
if the initialization closure panics.

## Lock Backends (`LockPolicy` & `SharingPolicy`)

- **`Atomic`**: A pure, portable spinlock based on `AtomicUsize`. Ideal for
`#![no_std]` and extremely short critical sections.
- **`Os`**: OS-specific high-performance locks. Uses `futex` on Linux,
`pthread_rwlock_t` on macOS, and `SRWLOCK` on Windows. Automatically parks
threads in the kernel on contention.
- **`Fs`**: A filesystem-based advisory lock using `flock(2)`. Useful for
cross-process synchronization.
- **`Irq`**: A bare-metal lock that disables hardware interrupts (IRQs) upon
acquisition. Essential for kernel development to prevent interrupt-handler
deadlocks.
- **`Nested<L1, L2>`**: A composite lock that strictly enforces acquisition
order (`L1` then `L2`) and reverse release order, preventing deadlocks at
compile time.
- **`Shield<L>`**: A wrapper that prevents writer starvation in read-write
locks by yielding readers (**sh**ared accessors y**ield**) the resource from
new readers when a writer is waiting.

## Retry Backends (`RetryPolicy`)

- **`Busy`**: Executes architecture-specific CPU pause instructions
(`core::hint::spin_loop()`).
- **`Yield`**: Cooperatively yields the current thread to the OS scheduler
(`std::thread::yield_now()`).
