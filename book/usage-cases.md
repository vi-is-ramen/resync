# 4. Usage Cases & Possibilities

## Case A: The Standard Library Replacement

For typical applications where you just want a fast mutex:

```rust
use resync::Mutex;

let mutex = Mutex::<i32>::new(42);
let guard = mutex.lock().unwrap();
```

Under the hood: Uses `lock::Os` (e.g. futexes/SRWLOCK) and `retry::Yield` (if std is enabled).

# Case B: Embedded / `no_std` Environments

In kernel modules or microcontrollers, yielding to an OS thread scheduler
is impossible. By disabling the std feature, Resync automatically
swaps the default retry strategy to `retry::Busy`, which issues
architecture-specific pause instructions (like `PAUSE` on x86 or `YIELD`
on ARM) to reduce power consumption and bus contention during tight loops.

```toml
# Cargo.toml
[dependencies]
resync = { version = "...", default-features = false }
```

# Case C: High-Contention Customization

If you know a specific lock will experience extreme contention, a standard
busy-wait might cause CPU starvation. You can implement a custom
`RetryPolicy` with exponential backoff and plug it into the
Mutex:

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
