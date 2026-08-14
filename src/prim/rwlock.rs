//! A readers-writer lock that uses a spin strategy.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use crate::{IShare, ISpin, LockStatus};

/// A readers-writer lock primitive that protects a value of type `T`.
#[allow(missing_debug_implementations)]
pub struct RwLock<
    T,
    L: IShare = crate::share::DefaultShare,
    S: ISpin = crate::spin::DefaultSpin,
> {
    inner: UnsafeCell<T>,
    lock:  L,
    spin:  S,
}

unsafe impl<T, L: IShare, S: ISpin> core::marker::Sync for RwLock<T, L, S> {}
unsafe impl<T, L: IShare, S: ISpin> core::marker::Send for RwLock<T, L, S> {}

impl<T: core::default::Default, L: IShare, S: ISpin> core::default::Default
    for RwLock<T, L, S>
{
    fn default() -> Self
    {
        Self {
            inner: UnsafeCell::new(T::default()),
            lock:  L::default(),
            spin:  S::default(),
        }
    }
}

/// A guard that provides immutable (reader) access.
#[allow(missing_debug_implementations)]
pub struct RwRef<'a, T, L: IShare, S: ISpin>
{
    data: *const T,
    lock: &'a RwLock<T, L, S>,
}

/// A guard that provides mutable (writer) access.
#[allow(missing_debug_implementations)]
pub struct RwMut<'a, T, L: IShare, S: ISpin>
{
    data: *mut T,
    lock: &'a RwLock<T, L, S>,
}

impl<'a, T, L: IShare, S: ISpin> core::ops::Drop for RwRef<'a, T, L, S>
{
    fn drop(&mut self)
    {
        self.lock.lock.free_share();
    }
}

impl<'a, T, L: IShare, S: ISpin> core::ops::Drop for RwMut<'a, T, L, S>
{
    fn drop(&mut self)
    {
        self.lock.lock.free();
    }
}

impl<'a, T, L: IShare, S: ISpin> Deref for RwRef<'a, T, L, S>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L: IShare, S: ISpin> Deref for RwMut<'a, T, L, S>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        unsafe { self.data.as_ref_unchecked() }
    }
}

impl<'a, T, L: IShare, S: ISpin> DerefMut for RwMut<'a, T, L, S>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        unsafe { self.data.as_mut_unchecked() }
    }
}

impl<T, L: IShare, S: ISpin> RwLock<T, L, S>
{
    /// Creates a new `RwLock` protecting the given `value`.
    pub fn new(value: T) -> Self
    {
        Self {
            inner: UnsafeCell::new(value),
            lock:  L::default(),
            spin:  S::default(),
        }
    }

    /// Attempts to acquire a reader lock without blocking.
    pub fn try_read(&self) -> Option<RwRef<'_, T, L, S>>
    {
        match self.lock.try_share(0)
        {
            Ok(LockStatus::Done) => Some(RwRef {
                data: self.inner.get(),
                lock: self,
            }),
            _ => None,
        }
    }

    /// Acquires a reader lock, spinning until available.
    pub fn read(&self) -> Option<RwRef<'_, T, L, S>>
    {
        let mut iteration = 0usize;

        loop
        {
            iteration += 1;

            match self.lock.try_share(iteration)
            {
                Ok(LockStatus::Done) =>
                {
                    return Some(RwRef {
                        data: self.inner.get(),
                        lock: self,
                    });
                },
                Ok(LockStatus::Fail) =>
                {
                    if self.spin.spin().is_err()
                    {
                        return None;
                    }
                },
                Err(_) => return None,
            }
        }
    }

    /// Attempts to acquire a writer lock without blocking.
    pub fn try_write(&self) -> Option<RwMut<'_, T, L, S>>
    {
        match self.lock.try_lock(0)
        {
            Ok(LockStatus::Done) => Some(RwMut {
                data: self.inner.get(),
                lock: self,
            }),
            _ => None,
        }
    }

    /// Acquires a writer lock, spinning until available.
    pub fn write(&self) -> Option<RwMut<'_, T, L, S>>
    {
        let mut iteration = 0usize;

        loop
        {
            iteration += 1;

            match self.lock.try_lock(iteration)
            {
                Ok(LockStatus::Done) =>
                {
                    return Some(RwMut {
                        data: self.inner.get(),
                        lock: self,
                    });
                },
                Ok(LockStatus::Fail) =>
                {
                    if self.spin.spin().is_err()
                    {
                        return None;
                    }
                },
                Err(_) => return None,
            }
        }
    }
}
