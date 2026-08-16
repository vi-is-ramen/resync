# 5. Usage Cases & Full Examples

## Case A: The Fair Read-Write Lock (Preventing Writer Starvation)

In standard `std::sync::RwLock` or basic atomic RW locks, a continuous stream
of readers can starve a waiting writer indefinitely. Resync solves this with
the `Shield` battery, which wraps any `SharingPolicy` and blocks new readers
the moment a writer starts waiting.

```rust
use resync::{Sharex, lock::{Os, Shield}, retry::Yield};

// Define a fair RW lock type
type FairRwLock<T> = Sharex<T, Shield<Os>, Yield>;

let lock = FairRwLock::new(vec![1, 2, 3]);

// Readers can proceed concurrently
let r1 = lock.read().unwrap();

// If a writer tries to acquire and fails, Shield increments a pending
// counter. Subsequent readers will receive `LockStatus::Fail` and yield,
// guaranteeing the writer gets the lock as soon as `r1` is dropped.
```

## Case B: Thread Pool Initialization with `Gate`

When spawning a pool of worker threads, you often want them to block until the
main thread finishes setting up the environment. Using a `std::sync::Barrier`
requires knowing the exact number of threads upfront and is single-use. Using
channels introduces allocation overhead.

`Gate` starts closed (via the `NewLocked` trait), ensuring no thread can slip
through before the setup is done (preventing TOCTOU races).

```rust
use resync::{Gate, lock::Os, retry::Yield};
use std::sync::Arc;
use std::thread;

fn main() {
    // Gate is CLOSED by default.
    let gate = Arc::new(Gate::<Os, Yield>::new());
    
    let workers: Vec<_> = (0..8).map(|id| {
        let g = Arc::clone(&gate);
        thread::spawn(move || {
            // All 8 threads block here immediately.
            g.wait().unwrap();
            println!("Worker {id} is processing!");
        })
    }).collect();

    // Main thread does heavy setup...
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Unleash all workers simultaneously.
    gate.open();

    for w in workers { w.join().unwrap(); }
}
```

## Case C: Bare-Metal Kernel Development (`no_std` + `Irq`)

In OS kernel development, if a thread holds a spinlock and gets interrupted by
a hardware IRQ, and the IRQ handler tries to acquire the *same* spinlock, the
system deadlocks. 

Resync provides the `Irq` lock policy, which automatically saves the CPU flags,
disables interrupts on acquisition, and restores them on release.

```rust,ignore
// In a #![no_std] kernel environment
use resync::{Mutex, lock::Irq, retry::Busy};

// The lock protects per-CPU data.
static KERNEL_STATE: Mutex<u32, Irq, Busy> = Mutex::new(0);

fn thread_context() {
    // Interrupts are disabled while this guard is alive.
    let mut state = KERNEL_STATE.lock().unwrap();
    *state += 1;
}

fn hardware_interrupt_handler() {
    // Safe to acquire the lock here, because thread_context 
    // guaranteed interrupts were disabled before taking it.
    let mut state = KERNEL_STATE.lock().unwrap();
    *state += 1;
}
```

## Case D: Graceful Recovery from Poisoning

Unlike `std::sync::Mutex` which forces you to `unwrap()` or manually extract
the inner error, Resync's `AcquireError` allows you to match on the exact
reason of failure, including timeouts from custom `RetryPolicy`
implementations.

```rust
use resync::{Mutex, AcquireError, lock::Os, retry::Yield};

let mutex = Mutex::<i32>::new(42);

match mutex.lock() {
    Ok(guard) => println!("Data is safe: {}", *guard),
    Err(AcquireError::Poisoned(err)) => {
        println!("Thread panicked! Inspecting corrupted data...");
        let mut guard = err.into_inner();
        *guard = 0; // Manually repair the state
        unsafe { mutex.clear_poison(); }
    },
    Err(AcquireError::Retry(timeout_err)) => {
        eprintln!("Lock acquisition timed out: {}", timeout_err);
    },
    Err(AcquireError::Lock(os_err)) => {
        eprintln!("Fatal OS error: {}", os_err);
    }
}
```
