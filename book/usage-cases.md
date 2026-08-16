# 4. Usage Cases & Possibilities

## Case A: The Standard Library Replacement
For typical applications where you just want a fast mutex:

```rust
use resync::Mutex;

let mutex = Mutex::<i32>::new(42);
let guard = mutex.lock().unwrap();
```
Under the hood: Uses `lock::Os` (e.g. futexes/SRWLOCK) and `retry::Yield`
(if std is enabled, `retry::Busy` otherwise).

## Case B: Embedded / `no_std` Environments
In kernel modules or microcontrollers, yielding to an OS thread scheduler
is impossible. By disabling the `std` feature, Resync automatically swaps
the default retry strategy to `retry::Busy`, which issues architecture-specific
pause instructions (like `PAUSE` on x86 or `YIELD` on ARM) to reduce power
consumption and bus contention during tight loops.

```toml
# Cargo.toml
[dependencies]
resync = { version = "...", default-features = false }
```

## Case C: High-Contention Customization
If you know a specific lock will experience extreme contention, a standard
busy-wait might cause CPU starvation. You can implement a custom `RetryPolicy`
with exponential backoff and plug it into the Mutex:

```rust
use resync::lock::Atomic;
use resync::{RetryPolicy, Mutex, RetryResult};

struct BackoffRetry(u32);

impl Default for BackoffRetry {
    fn default() -> Self { Self(0) }
}

impl RetryPolicy for BackoffRetry
{
    type Error = core::convert::Infallible;

    fn retry(&self, _current_iteration: usize) -> RetryResult<Self::Error>
    {
        // Custom backoff logic here...
        Ok(())
    }
}

pub type BackoffMutex<T, L> = Mutex<T, L, BackoffRetry>;

// Explicitly define the types
let mutex = BackoffMutex::<u32, Atomic>::new(0);
```

## Case D: Controlling Thread Flow with `Gate`
A `Gate` acts as a controllable barrier. By default, it starts in the **closed**
state (using the `NewLocked` trait), blocking any threads that call `wait()`. This
is perfect for initializing a pool of workers that must not start processing until
the setup phase is complete.

```rust
use resync::{Gate, lock::Atomic, retry::Yield};
use std::sync::Arc;
use std::thread;

let gate = Arc::new(Gate::<Atomic, Yield>::new()); // Starts closed

let workers: Vec<_> = (0..4).map(|i| {
    let g = Arc::clone(&gate);
    thread::spawn(move || {
        g.wait().unwrap(); // Blocks here
        println!("Worker {i} started!");
    })
}).collect();

// ... perform setup ...
gate.open(); // Unblocks all workers simultaneously

for w in workers { w.join().unwrap(); }
```

## Case E: Resource Pooling with `Semaphore`
Use a `Semaphore` to limit concurrent access to a pool of resources, such as
database connections or worker threads.

```rust
use resync::{Semaphore, lock::Os, retry::Yield};
use std::sync::Arc;
use std::thread;

// Allow up to 3 concurrent connections
let sem = Arc::new(Semaphore::<Os, Yield>::new(3));

let handles: Vec<_> = (0..10).map(|i| {
    let s = Arc::clone(&sem);
    thread::spawn(move || {
        s.acquire().unwrap();
        println!("Task {i} acquired a connection");
        // ... do work ...
        s.release().unwrap();
    })
}).collect();
```

## Case F: Event Waiting with `Condvar`
`Condvar` allows threads to wait for a specific condition to become true, fully
respecting the poisoning semantics of the associated `Mutex`.

```rust
use resync::{Mutex, Condvar, AcquireError};
use std::sync::Arc;
use std::thread;

let pair = Arc::new((Mutex::new(false), Condvar::new()));
let pair2 = Arc::clone(&pair);

thread::spawn(move || {
    let (lock, cvar) = &*pair2;
    let mut started = lock.lock().unwrap();
    *started = true;
    cvar.notify_one();
});

let (lock, cvar) = &*pair;
let mut started = lock.lock().unwrap();
while !*started {
    started = cvar.wait(started, lock).unwrap();
}
```
