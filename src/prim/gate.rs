//! A one-shot synchronization primitive that blocks threads until opened.

use crate::{ILock, ISpin, LockResult, SpinResult};

/// A gate is a one-shot synchronization primitive that allows threads to
/// block until another thread signals them via [`open`](Gate::open).
///
/// Once opened, the gate remains open forever.
///
/// # Semantics
///
/// A gate is "closed" when the inner lock is held, and "open" when the
/// inner lock is free. `wait()` spins until the lock becomes free, while
/// `open()` releases the lock.
///
/// # Type Parameters
/// - `L`: the lock type (must implement [`ILock`])
/// - `S`: the spin strategy (must implement [`ISpin`])
///
/// # Examples
///
/// ```ignore
/// use resync::lock::Atomic;
/// use resync::spin::Busy;
/// use resync::Gate;
/// use std::thread;
/// use std::sync::Arc;
///
/// let gate = Arc::new(Gate::<Atomic, Busy>::new().unwrap());
/// let g = Arc::clone(&gate);
///
/// let handle = thread::spawn(move || {
///     // Do some work...
///     g.open();
/// });
///
/// gate.wait().unwrap();
/// handle.join().unwrap();
/// ```
#[allow(missing_debug_implementations)]
pub struct Gate<
    L: ILock = crate::lock::Atomic,
    S: ISpin = crate::spin::DefaultSpin,
> {
    lock: L,
    spin: S,
}

impl<L: ILock, S: ISpin> Gate<L, S>
{
    /// Creates a new closed gate.
    ///
    /// The gate is initialized in the "closed" state by acquiring the inner
    /// lock. Subsequent calls to `wait()` will spin until `open()` is called.
    pub fn new() -> Option<Self>
    {
        let this = Self {
            lock: L::default(),
            spin: S::default(),
        };

        if let LockResult::Done = this.lock.try_lock(0)
        {
            Some(this)
        }
        else
        {
            None
        }
    }

    /// Blocks the current thread until the gate is opened.
    ///
    /// This method spins until the inner lock becomes free (i.e., `open()`
    /// has been called). If the gate is already open, this returns immediately.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` if the spin strategy returns `Abort` or the lock
    /// reports `Abort`.
    #[allow(clippy::result_unit_err)]
    pub fn wait(&self) -> Result<(), ()>
    {
        loop
        {
            match self.lock.fake_lock()
            {
                LockResult::Done => return Ok(()),
                LockResult::Fail => match self.spin.spin()
                {
                    SpinResult::Ok => continue,
                    SpinResult::Abort => return Err(()),
                },
                LockResult::Abort => return Err(()),
            }
        }
    }

    /// Opens the gate, releasing all waiting threads.
    ///
    /// This releases the inner lock, allowing all threads blocked in `wait()`
    /// to proceed. Subsequent calls to `wait()` will return immediately.
    ///
    /// If the gate is already open, this does nothing.
    pub fn open(&self)
    {
        self.lock.free();
        self.lock.wake_all();
    }
}

impl<L: ILock, S: ISpin> core::default::Default for Gate<L, S>
{
    fn default() -> Self
    {
        Self::new().unwrap()
    }
}

#[cfg(all(test, feature = "std"))]
mod tests
{
    use super::*;
    use crate::lock::Atomic;
    use crate::spin::Busy;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn gate_new_is_closed()
    {
        let gate = Gate::<Atomic, Busy>::new().unwrap();
        assert!(!matches!(gate.lock.fake_lock(), LockResult::Done));
    }

    #[test]
    fn gate_open_opens()
    {
        let gate = Gate::<Atomic, Busy>::new().unwrap();
        gate.open();
        assert!(matches!(gate.lock.fake_lock(), LockResult::Done));
    }

    #[test]
    fn gate_wait_returns_after_open()
    {
        let gate = Arc::new(Gate::<Atomic, Busy>::new().unwrap());
        let g = Arc::clone(&gate);
        let handle = thread::spawn(move || {
            g.open();
        });
        gate.wait().unwrap();
        handle.join().unwrap();
        assert!(matches!(gate.lock.fake_lock(), LockResult::Done));
    }

    #[test]
    fn gate_try_wait_works()
    {
        let gate = Gate::<Atomic, Busy>::new().unwrap();
        assert!(!matches!(gate.lock.fake_lock(), LockResult::Done));
        gate.open();
        assert!(matches!(gate.lock.fake_lock(), LockResult::Done));
    }

    #[test]
    fn gate_multiple_waiters()
    {
        let gate = Arc::new(Gate::<Atomic, Busy>::new().unwrap());
        let mut handles = vec![];
        for _ in 0..3
        {
            let g = Arc::clone(&gate);
            handles.push(thread::spawn(move || {
                g.wait().unwrap();
            }));
        }
        thread::sleep(std::time::Duration::from_millis(50));
        gate.open();
        for h in handles
        {
            h.join().unwrap();
        }
    }

    #[test]
    fn gate_open_is_idempotent()
    {
        let gate = Gate::<Atomic, Busy>::new().unwrap();
        gate.open();
        gate.open();
        gate.open();
        assert!(matches!(gate.lock.fake_lock(), LockResult::Done));
    }

    #[test]
    fn gate_wait_after_open_returns_immediately()
    {
        let gate = Gate::<Atomic, Busy>::new().unwrap();
        gate.open();
        // All subsequent waits should return immediately
        gate.wait().unwrap();
        gate.wait().unwrap();
        gate.wait().unwrap();
    }
}
