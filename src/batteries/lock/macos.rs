use crate::traits::{LockPolicy, SharingPolicy};
use crate::{LockResult, LockStatus};

#[allow(missing_debug_implementations)]
pub struct Os
{
    rwlock: core::cell::UnsafeCell<libc::pthread_rwlock_t>,
}

impl Os
{
    pub fn new() -> Self
    {
        let rwlock = core::cell::UnsafeCell::new(unsafe {
            let mut rwlock: libc::pthread_rwlock_t = core::mem::zeroed();
            let result =
                libc::pthread_rwlock_init(&mut rwlock, core::ptr::null());
            debug_assert_eq!(result, 0, "pthread_rwlock_init failed");
            rwlock
        });

        Self { rwlock }
    }
}

impl core::default::Default for Os
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl Drop for Os
{
    fn drop(&mut self)
    {
        unsafe {
            libc::pthread_rwlock_destroy(self.rwlock.get());
        }
    }
}

unsafe impl Send for Os {}
unsafe impl Sync for Os {}

unsafe impl LockPolicy for Os
{
    type Error = core::convert::Infallible;

    unsafe fn try_lock(&self, _current_iteration: usize) -> LockResult
    {
        let result =
            unsafe { libc::pthread_rwlock_trywrlock(self.rwlock.get()) };

        if result == 0
        {
            LockResult::Ok(LockStatus::Done)
        }
        else
        {
            LockResult::Ok(LockStatus::Fail)
        }
    }

    fn get_state(&self) -> LockResult
    {
        // pthread_rwlock doesn't provide a non-modifying check
        LockResult::Ok(LockStatus::Done)
    }

    unsafe fn free(&self)
    {
        unsafe {
            libc::pthread_rwlock_unlock(self.rwlock.get());
        }
    }

    fn wake_all(&self)
    {
        // pthread_rwlock handles waking automatically
    }
}

unsafe impl SharingPolicy for Os
{
    fn try_share(&self, _current_iteration: usize) -> LockResult
    {
        let result =
            unsafe { libc::pthread_rwlock_tryrdlock(self.rwlock.get()) };

        if result == 0
        {
            LockResult::Ok(LockStatus::Done)
        }
        else
        {
            LockResult::Ok(LockStatus::Fail)
        }
    }

    fn free_share(&self)
    {
        unsafe {
            libc::pthread_rwlock_unlock(self.rwlock.get());
        }
    }

    fn wake_readers(&self)
    {
        // pthread_rwlock handles waking automatically
    }
}
