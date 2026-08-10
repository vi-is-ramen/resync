//! # The Resync Guidebook
//!
//! Welcome to the comprehensive guide for Resync. This document covers the
//! library's philosophy, core concepts, advanced usage patterns, design
//! decisions, and inherent limitations.
//!
//! ## 1. Philosophy: The "LEGO" Approach
//!
//! Unlike standard library synchronization primitives that provide a monolithic
//! `Mutex` or `RwLock`, `resync` treats synchronization as a composition of
//! smaller, independent behaviors.
//!
//! A blocking mutex is essentially two behaviors combined:
//! 1. **Acquisition:** How do we atomically claim ownership of a resource?
//! 2. **Waiting:** What do we do while the resource is held by someone else?
//!
//! Resync decouples these concerns into the [`crate::ILock`] and
//! [`crate::ISpin`] traits. This allows you to mix and match atomic acquisition
//! strategies with different spin-wait strategies at compile time, tailoring
//! the primitive exactly to your performance and environment constraints.
//!
//! ## 2. Core Traits
//!
//! ### [`crate::ILock`]: The Acquisition Strategy
//! The [`crate::ILock`] trait defines the raw mechanics of claiming a resource.
//! It is intentionally minimal and strictly atomic.
//!
//! ### [`crate::ISpin`]: The Waiting Strategy
//! The [`crate::ISpin`] trait defines what the CPU should do when an
//! [`crate::ILock`] reports contention. This could be a tight CPU pause loop
//! ([`crate::spin::Busy`]) or yielding to the OS scheduler
//! ([`crate::spin::Os`]).
//!
//! ### [`crate::Mutex`]: The Composition
//! The [`crate::Mutex`] struct binds an [`crate::ILock`], an [`crate::ISpin`],
//! and the protected data (`T`) together, providing a safe, RAII-based
//! interface ([`crate::MutexGuard`]).
//!
//! ## 3. Design Decisions
//!
//! ### Why no `is_locked()` or `is_free()` in [`crate::ILock`]?
//! A common question when designing lock APIs is: *"Why can't I check if the
//! lock is currently held before trying to acquire it?"*
//!
//! In concurrent programming, checking a state and then acting on it introduces
//! a **Time-of-Check to Time-of-Use (TOCTOU)** race condition. Consider this
//! hypothetical anti-pattern:
//!
//! ```rust,ignore
//! // HYPOTHETICAL BAD CODE
//! if !lock.is_locked() {
//!     // Another thread could acquire the lock RIGHT HERE
//!     lock.lock();
//! }
//! ```
//!
//! By the time you call `lock()` after checking `is_locked()`, the state may
//! have already changed. The check is not only wasted CPU cycles, but it gives
//! a false sense of security and predictability.
//!
//! By omitting state-querying methods, Resync forces you to use
//! [`crate::ILock::try_lock`], which is an **atomic check-and-acquire**
//! operation. The result of the operation tells you the state *at the exact
//! moment* the atomic instruction executed, eliminating TOCTOU bugs by design.
//!
//! ### Why `crate::LockResult` and `crate::SpinResult` instead of `bool`?
//! Standard library locks often return `bool` or `Result<T, PoisonError>`.
//! `resync` uses explicit enums ([`crate::LockResult`] and
//! [`crate::SpinResult`]) to handle three distinct states without relying on
//! panics:
//!
//! * **`Done` / `Ok`:** Success.
//! * **`Fail`:** Contention. The lock is held, but the system is healthy. The
//!   caller should spin or yield.
//! * **`Abort`:** Unrecoverable error. The lock is "poisoned", the underlying
//!   hardware failed, or the spin strategy detected a timeout. This allows
//!   `no_std` environments to handle catastrophic failures gracefully without
//!   unwinding panics.
//!
//! ### Why is [`crate::ILock::free`] idempotent?
//! Calling `free()` on an already free lock is a guaranteed no-op. This
//! dramatically simplifies the implementation of composite locks (like
//! [`crate::lock::Nested`]) and error-handling paths. If an abort occurs
//! halfway through acquiring a nested lock, the cleanup code can safely call
//! `free()` on all inner locks without needing to track which specific ones
//! were successfully acquired.
//!
//! ### Why [`crate::lock::Nested`]?
//! Deadlocks often occur when multiple locks are acquired in inconsistent
//! orders across different threads. [`crate::lock::Nested`] enforces a strict,
//! deterministic acquisition order (`L1` then `L2`) and a reverse release
//! order (`L2` then `L1`). This provides a compile-time building block for
//! safe multi-resource locking.
//!
//! ## 4. Usage Cases & Possibilities
//!
//! ### Case A: The Standard Library Replacement
//! For typical applications where you just want a fast mutex:
//!
//! ```rust
//! use resync::Mutex;
//!
//! let mutex = Mutex::<i32>::new(42);
//! let guard = mutex.lock().unwrap();
//! ```
//! *Under the hood:* Uses [`crate::lock::Atomic`] and [`crate::spin::Os`] (if
//! `std` is enabled).
//!
//! ### Case B: Embedded / `no_std` Environments
//! In kernel modules or microcontrollers, yielding to an OS thread scheduler
//! is impossible. By disabling the `std` feature, Resync automatically
//! swaps the default spin strategy to [`crate::spin::Busy`], which issues
//! architecture-specific pause instructions (like `PAUSE` on x86 or `YIELD`
//! on ARM) to reduce power consumption and bus contention during tight loops.
//!
//! ```toml
//! # Cargo.toml
//! [dependencies]
//! resync = { version = "...", default-features = false }
//! ```
//!
//! ### Case C: High-Contention Customization
//! If you know a specific lock will experience extreme contention, a standard
//! busy-wait might cause CPU starvation. You can implement a custom
//! [`crate::ISpin`] with exponential backoff and plug it into the
//! [`crate::Mutex`]:
//!
//! ```rust
//! use resync::lock::Atomic;
//! use resync::{ISpin, Mutex, SpinResult};
//!
//! struct BackoffSpin(u32);
//!
//! impl ISpin for BackoffSpin
//! {
//!     fn spin(&self) -> SpinResult
//!     {
//!         // Custom backoff logic here...
//!         SpinResult::Ok
//!     }
//! }
//!
//! // Explicitly define the types
//! let mutex: Mutex<u32, Atomic, BackoffSpin> = Mutex::new(0);
//! ```
//!
//! ## 5. Limitations and Caveats
//!
//! While Resync is highly flexible, it is important to understand its
//! boundaries:
//!
//! * **No Async/Await Support:** Resync is strictly designed for synchronous,
//!   thread-based, or interrupt-level synchronization. It does not integrate
//!   with Rust's `Waker` or `Context` APIs. Using spin-locks inside an async
//!   executor will block the executor thread and starve other futures.
//! * **Fairness is Not Guaranteed:** The default [`crate::lock::Atomic`] uses a
//!   simple `compare_exchange`. Under extreme contention, this can lead to
//!   thread starvation (where one thread repeatedly wins the race). If strict
//!   fairness is required, you must implement a custom [`crate::ILock`] (e.g.,
//!   a ticket lock or MCS lock).
//! * **Nightly vs. Stable:** On stable Rust, traits and default implementations
//!   cannot be `const`. If you require `const` initialization of your locks in
//!   static variables, you must compile your crate with a **nightly**
//!   toolchain. `resync` will automatically detect the nightly channel and
//!   enable `const_trait_impl` and `const_default`.
//! * **Spin-Loop Starvation:** If the thread holding the lock is preempted by
//!   the OS while a waiting thread is executing a [`crate::spin::Busy`] loop,
//!   the waiting thread will burn CPU cycles until the OS reschedules the
//!   holder. Always prefer [`crate::spin::Os`] in user-space applications
//!   unless you are certain the critical section is shorter than a context
//!   switch.
//!
//! ## 6. Summary
//!
//! Resync gives you the raw materials to build exactly the synchronization
//! primitive you need, while enforcing safe, race-free API boundaries. By
//! understanding the distinction between *acquisition* and *waiting*, you
//! can optimize your concurrent code for any environment, from bare-metal
//! microcontrollers to high-throughput user-space servers.
