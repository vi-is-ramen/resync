//! A lazy initialization primitive.
//!
//! This module provides the [`Lazy`] struct, which allows lazy initialization
//! of values. It is similar to [`std::sync::LazyLock`] and `lazy_static`, but
//! built using `resync`'s composable [`LockPolicy`](crate::traits::LockPolicy),
//! [`RetryPolicy`](crate::traits::RetryPolicy), and
//! [`PoisonPolicy`](crate::traits::PoisonPolicy) traits.
//!
//! # Examples
//!
//! ```rust
//! use resync::Lazy;
//! use std::collections::HashMap;
//!
//! static HASHMAP: Lazy<HashMap<i32, i32>> = Lazy::new(|| {
//!     let mut m = HashMap::new();
//!     m.insert(1, 2);
//!     m
//! });
//!
//! fn main()
//! {
//!     // First access initializes the HashMap
//!     assert_eq!(HASHMAP.get(&1), Some(&2));
//! }
//! ```

use crate::LockStatus;
use crate::traits::{LockPolicy, PoisonPolicy, RetryPolicy};
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::sync::atomic::{AtomicU8, Ordering};

const UNTOUCHED: u8 = 0;
const UNINIT: u8 = 1;
const INITIALIZING: u8 = 2;
const DONE: u8 = 3;

/// A value which is initialized on the first access.
///
/// This type is similar to [`std::sync::LazyLock`] and `lazy_static`, but uses
/// `resync`'s composable policies for synchronization.
///
/// # Type Parameters
///
/// - `T`: The type of the value to be lazily initialized.
/// - `F`: The initialization function type.
/// - `L`: The [`LockPolicy`] used for synchronization during initialization.
/// - `R`: The [`RetryPolicy`] used when waiting for initialization.
/// - `P`: The [`PoisonPolicy`] used to handle panics during initialization.
///
/// # Poisoning
///
/// If the initialization function panics, the `Lazy` primitive becomes
/// poisoned (if the configured [`PoisonPolicy`] supports it, e.g.,
/// [`crate::poison::StdPoison`]). Subsequent accesses will panic with a
/// message indicating that the initialization failed.
#[allow(missing_debug_implementations)]
pub struct Lazy<
    T,
    F = fn() -> T,
    L = crate::lock::DefaultLock,
    R = crate::retry::DefaultRetry,
    P = crate::poison::DefaultPoison,
> where
    F: FnOnce() -> T,
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    state:  AtomicU8,
    init:   UnsafeCell<Option<F>>,
    data:   UnsafeCell<MaybeUninit<T>>,
    lock:   UnsafeCell<MaybeUninit<L>>,
    retry:  UnsafeCell<MaybeUninit<R>>,
    poison: UnsafeCell<MaybeUninit<P::State>>,
}

// SAFETY:
// `Lazy` ensures that initialization happens exactly once, and subsequent
// accesses are read-only. It is safe to share across threads.
unsafe impl<T, F, L, R, P> core::marker::Sync for Lazy<T, F, L, R, P>
where
    T: Send + Sync,
    F: FnOnce() -> T + Send + Sync,
    L: LockPolicy + Sync,
    R: RetryPolicy + Sync,
    P: PoisonPolicy + Sync,
{
}

// SAFETY:
// `Lazy` can be safely moved between threads.
unsafe impl<T, F, L, R, P> core::marker::Send for Lazy<T, F, L, R, P>
where
    T: Send,
    F: FnOnce() -> T + Send,
    L: LockPolicy + Send,
    R: RetryPolicy + Send,
    P: PoisonPolicy + Send,
{
}

impl<T, F, L, R, P> Lazy<T, F, L, R, P>
where
    F: FnOnce() -> T,
    L: LockPolicy,
    R: RetryPolicy,
    P: PoisonPolicy,
{
    /// Creates a new `Lazy` value with the given initialization function.
    ///
    /// This is a `const` function, allowing the `Lazy` to be used in static
    /// variables.
    pub const fn new(init: F) -> Self
    {
        Self {
            state:  AtomicU8::new(UNTOUCHED),
            init:   UnsafeCell::new(Some(init)),
            data:   UnsafeCell::new(MaybeUninit::uninit()),
            lock:   UnsafeCell::new(MaybeUninit::uninit()),
            retry:  UnsafeCell::new(MaybeUninit::uninit()),
            poison: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Forces the evaluation of this lazy value and returns a reference to
    /// the result.
    ///
    /// This is equivalent to the `Deref` implementation, but allows explicit
    /// calls.
    pub fn force(&self) -> &T
    where
        L: LockPolicy + Default,
        R: RetryPolicy + Default,
    {
        match self.state.load(Ordering::Acquire)
        {
            DONE =>
            unsafe { (*self.data.get()).assume_init_ref() },
            UNTOUCHED => self.initialize_from_untouched(),
            UNINIT => self.wait_for_uninit(),
            INITIALIZING => self.wait_for_initializing(),
            _ => unreachable!(),
        }
    }

    fn initialize_from_untouched(&self) -> &T
    where
        L: LockPolicy + Default,
        R: RetryPolicy + Default,
    {
        // Try to transition from UNTOUCHED to UNINIT
        match self.state.compare_exchange(
            UNTOUCHED,
            UNINIT,
            Ordering::Acquire,
            Ordering::Relaxed,
        )
        {
            Ok(_) =>
            {
                // We won the race, initialize L/R/P
                unsafe {
                    (*self.lock.get()).write(L::default());
                    (*self.retry.get()).write(R::default());
                    (*self.poison.get()).write(P::new_state());
                }

                // Transition to INITIALIZING
                self.state.store(INITIALIZING, Ordering::Release);

                // Now initialize the actual value
                self.do_initialize()
            },
            Err(_) =>
            {
                // Someone else is initializing, wait for them
                self.wait_for_uninit()
            },
        }
    }

    fn wait_for_uninit(&self) -> &T
    where
        L: LockPolicy + Default,
        R: RetryPolicy + Default,
    {
        let retry = R::default();
        let mut iterations = 0usize;

        // Spin until state becomes INITIALIZING or DONE
        loop
        {
            match self.state.load(Ordering::Acquire)
            {
                INITIALIZING => return self.wait_for_initializing(),
                DONE =>
                {
                    // Check if poisoned
                    if let Some(poison) = self.get_poison_state()
                        && P::is_poisoned(poison)
                    {
                        panic!("Lazy initialization previously panicked");
                    }
                    return unsafe { (*self.data.get()).assume_init_ref() };
                },
                _ =>
                {
                    iterations += 1;
                    if retry.retry(iterations).is_err()
                    {
                        panic!("Retry policy aborted on Lazy initialization");
                    }
                },
            }
        }
    }

    fn wait_for_initializing(&self) -> &T
    where
        L: LockPolicy + Default,
        R: RetryPolicy + Default,
    {
        // L/R/P are now initialized, use L to synchronize
        let lock = unsafe { (*self.lock.get()).assume_init_ref() };
        let retry = unsafe { (*self.retry.get()).assume_init_ref() };

        let mut iterations = 0usize;
        loop
        {
            match unsafe { lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done(meta)) =>
                {
                    // We got the lock, check if initialization is done
                    if self.state.load(Ordering::Acquire) == DONE
                    {
                        unsafe { lock.free(&meta) };

                        // Check if poisoned
                        if let Some(poison) = self.get_poison_state()
                            && P::is_poisoned(poison)
                        {
                            panic!("Lazy initialization previously panicked");
                        }
                        return unsafe { (*self.data.get()).assume_init_ref() };
                    }
                    else
                    {
                        // Still initializing, release and retry
                        unsafe { lock.free(&meta) };
                        iterations += 1;
                        if retry.retry(iterations).is_err()
                        {
                            panic!(
                                "Retry policy aborted on Lazy initialization"
                            );
                        }
                    }
                },
                Ok(LockStatus::Fail) =>
                {
                    // Lock is held by the initializing thread, retry
                    iterations += 1;
                    if retry.retry(iterations).is_err()
                    {
                        panic!("Retry policy aborted on Lazy initialization");
                    }
                },
                Err(_) =>
                {
                    panic!("Lock policy failed during Lazy initialization");
                },
            }
        }
    }

    fn do_initialize(&self) -> &T
    where
        L: LockPolicy + Default,
        R: RetryPolicy + Default,
    {
        let lock = unsafe { (*self.lock.get()).assume_init_ref() };
        let retry = unsafe { (*self.retry.get()).assume_init_ref() };
        let poison = unsafe { (*self.poison.get()).assume_init_ref() };

        // Try to acquire the lock
        let mut iterations = 0usize;
        let meta = loop
        {
            iterations += 1;
            match unsafe { lock.try_lock(iterations) }
            {
                Ok(LockStatus::Done(meta)) => break meta,
                Ok(LockStatus::Fail) =>
                {
                    if retry.retry(iterations).is_err()
                    {
                        panic!(
                            "Retry policy failed during Lazy initialization"
                        );
                    }
                },
                Err(_) =>
                {
                    panic!("Lock policy failed during Lazy initialization");
                },
            }
        };

        // Initialize the value
        let init_fn = unsafe { (*self.init.get()).take().unwrap() };

        // Use a guard to handle panics and ensure lock release
        struct InitGuard<'a, L: LockPolicy, P: PoisonPolicy>
        {
            lock:    &'a L,
            meta:    L::Meta,
            poison:  &'a P::State,
            state:   &'a AtomicU8,
            success: bool,
        }

        impl<'a, L: LockPolicy, P: PoisonPolicy> Drop for InitGuard<'a, L, P>
        {
            fn drop(&mut self)
            {
                unsafe { self.lock.free(&self.meta) };

                if !self.success
                {
                    P::on_drop(self.poison);
                }

                // Transition to DONE
                self.state.store(DONE, Ordering::Release);
            }
        }

        let mut guard = InitGuard::<L, P> {
            lock,
            meta,
            poison,
            state: &self.state,
            success: false,
        };

        let value = init_fn();
        unsafe { (*self.data.get()).write(value) };
        guard.success = true;

        drop(guard);

        unsafe { (*self.data.get()).assume_init_ref() }
    }

    fn get_poison_state(&self) -> Option<&P::State>
    {
        if self.state.load(Ordering::Acquire) == DONE
        {
            Some(unsafe { (*self.poison.get()).assume_init_ref() })
        }
        else
        {
            None
        }
    }

    /// Returns `true` if the lazy value has been initialized.
    pub fn is_initialized(&self) -> bool
    {
        self.state.load(Ordering::Acquire) == DONE
    }

    /// Returns `true` if the initialization panicked and the value is poisoned.
    pub fn is_poisoned(&self) -> bool
    {
        if let Some(poison) = self.get_poison_state()
        {
            P::is_poisoned(poison)
        }
        else
        {
            false
        }
    }
}

impl<T, F, L, R, P> Deref for Lazy<T, F, L, R, P>
where
    F: FnOnce() -> T,
    L: LockPolicy + Default,
    R: RetryPolicy + Default,
    P: PoisonPolicy,
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        self.force()
    }
}
