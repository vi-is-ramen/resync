//! A one-shot synchronization primitive that blocks threads until opened.

use crate::{ILock, ISpin, LockStatus};

/// A gate is a one-shot synchronization primitive.
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

        if let Ok(LockStatus::Done) = this.lock.try_lock(0)
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
    /// # Returns
    /// - `Ok(())`: gate is open
    /// - `Err(())`: spin aborted or error occurred
    #[allow(clippy::result_unit_err)]
    pub fn wait(&self) -> Result<(), ()>
    {
        loop
        {
            match self.lock.fake_lock()
            {
                Ok(LockStatus::Done) => return Ok(()),
                Ok(LockStatus::Fail) =>
                {
                    if self.spin.spin().is_err()
                    {
                        return Err(());
                    }
                },
                Err(_) => return Err(()),
            }
        }
    }

    /// Opens the gate, releasing all waiting threads.
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
