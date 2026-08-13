//! A one-shot synchronization primitive that blocks threads until opened.

use crate::{ILock, ISpin, LockResult, SpinResult};

/// A gate is a one-shot synchronization primitive that allows one or more
/// threads to block until another thread signals them via [`open`].
///
/// This is useful for scenarios where a single event must occur before
/// proceeding, such as:
/// - A kernel module waiting for a device to be ready.
/// - A thread waiting for initialisation to complete.
/// - One-time configuration or setup phases.
///
/// Once opened, the gate remains open forever. Attempts to wait after opening
/// return immediately.
///
/// # Examples
///
/// ```ignore
/// # use resync::spin::Os;
/// use resync::Gate;
/// use std::thread;
///
/// static gate: Gate<Os> = Gate::new();
/// let g = &gate;
///
/// let handle = thread::spawn(move || {
///     // Do some work...
///     g.open(); // Signal that work is done
/// });
///
/// // Block until the gate is opened
/// gate.wait();
/// // Proceed...
/// handle.join().unwrap();
/// ```
///
/// # Type Parameters
/// - `S`: the spin strategy used while waiting (must implement [`ISpin`]).
///   Defaults to [`crate::spin::DefaultSpin`].
///
/// # Limitations
/// - This is a spin-based primitive: waiting threads consume CPU cycles.
/// - It is one-shot; it cannot be reset.
///
/// [`open`]: Gate::open
// TODO: add generation semantics to allow reuse
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
    pub fn new() -> Option<Self>
    {
        let this = Self {
            lock: L::default(),
            spin: S::default(),
        };

        if let LockResult::Done = this.lock.try_lock()
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
    /// If the gate is already open, this returns immediately.
    ///
    /// # Errors
    ///
    /// This method returns [`Err`] if spin stratefy returned Fail.
    #[allow(clippy::result_unit_err)] // NOTE: intended
    pub fn wait(&self) -> Result<(), ()>
    {
        let result;

        loop
        {
            match self.lock.fake_lock()
            {
                LockResult::Done =>
                {
                    result = Ok(());
                    break
                },
                LockResult::Fail => match self.spin.spin()
                {
                    SpinResult::Ok => continue,
                    SpinResult::Abort =>
                    {
                        result = Err(());
                        break
                    },
                },
                LockResult::Abort =>
                {
                    result = Err(());
                    break
                },
            }
        }

        result
    }

    /// Opens the gate, releasing all waiting threads.
    ///
    /// Subsequent calls to `wait` will return immediately.
    ///
    /// If the gate is already open, this does nothing.
    pub fn open(&self)
    {
        self.lock.free();
    }

    // NOTE: no `is_open` or similar method because of TOCTOU danger.
    // NOTE: no `try_wait` method because of useless and TOCTOU danger.

    // TODO: Close/reset method
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
        gate.wait().unwrap(); // should block until opened
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
        // open after a small delay
        thread::sleep(std::time::Duration::from_millis(50));
        gate.open();
        for h in handles
        {
            h.join().unwrap();
        }
    }
}
