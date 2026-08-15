# 5. Limitations and Caveats

While Resync is highly flexible, it is important to understand its
boundaries:

- **No Async/Await Support:** Resync is strictly designed for synchronous,
thread-based, or interrupt-level synchronization. It does not integrate
with Rust's `Waker` or `Context` APIs. Using spin-locks inside an async
executor will block the executor thread and starve other futures.
- **Fairness is Not Guaranteed:** The default `lock::Atomic` uses a
simple `compare_exchange`. Under extreme contention, this can lead to
thread starvation (where one thread repeatedly wins the race). If strict
fairness is required, you must implement a custom `LockPolicy` (e.g.,
a ticket lock or MCS lock).
- **Nightly vs. Stable:** On stable Rust, traits and default implementations
cannot be `const`. If you require `const` initialization of your locks in
static variables, you must compile your crate with a **nightly**
toolchain. Resync will automatically detect the nightly channel and
enable `const_trait_impl` and `const_default`.
- **Spin-Loop Starvation:** If the thread holding the lock is preempted by
the OS while a waiting thread is executing a `retry::Busy` loop,
the waiting thread will burn CPU cycles until the OS reschedules the
holder. Always prefer `retry::Yield` in user-space applications
unless you are certain the critical section is shorter than a context
switch.
