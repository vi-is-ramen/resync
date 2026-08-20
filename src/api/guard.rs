use core::ops::{Deref, DerefMut};

/// A generic trait for readable data guards.
///
/// This trait abstracts the core guarding behavior of a mutex, decoupling the
/// *action* of read-guarding from the specific *implementation*.
// NOTE: This trait **must** be dyn-compatible by design.
pub trait Guard<T>
where Self: Deref<Target = T>
{
}

/// A generic trait for mutual data guards.
///
/// This trait abstracts the core guarding behavior of a mutex, decoupling the
/// *action* of mutual-guarding from the specific *implementation*.
///
/// ---
///
/// *Extends [`Guard`]*
// NOTE: This trait **must** be dyn-compatible by design.
pub trait GuardMut<T>
where Self: Guard<T> + DerefMut<Target = T>
{
}

#[cfg(std)]
impl<'a, T> Guard<T> for std::sync::MutexGuard<'a, T> {}
#[cfg(std)]
impl<'a, T> Guard<T> for std::sync::RwLockReadGuard<'a, T> {}
#[cfg(std)]
impl<'a, T> Guard<T> for std::sync::RwLockWriteGuard<'a, T> {}

#[cfg(std)]
impl<'a, T> GuardMut<T> for std::sync::MutexGuard<'a, T> {}
#[cfg(std)]
impl<'a, T> GuardMut<T> for std::sync::RwLockWriteGuard<'a, T> {}
