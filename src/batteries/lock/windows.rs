use crate::{ILock, IShare, LockResult};

#[allow(missing_debug_implementations)]
#[repr(transparent)]
pub struct Os
{
    srwlock: windows_sys::Win32::System::Threading::SRWLOCK,
}

impl Os
{
    pub fn new() -> Self
    {
        Self {
            srwlock: windows_sys::Win32::System::Threading::SRWLOCK {
                Ptr: core::ptr::null_mut(),
            },
        }
    }
}

impl core::default::Default for Os
{
    fn default() -> Self
    {
        Self::new()
    }
}

unsafe impl Send for Os {}
unsafe impl Sync for Os {}

unsafe impl ILock for Os
{
    type Error = core::convert::Infallible;

    fn try_lock(&self, _current_iteration: usize) -> LockResult
    {
        let result = unsafe {
            windows_sys::Win32::System::Threading::TryAcquireSRWLockExclusive(
                &self.srwlock as *const _ as *mut _,
            )
        };

        if result
        {
            LockResult::Ok(LockStatus::Done)
        }
        else
        {
            LockResult::Ok(LockStatus::Fail)
        }
    }

    fn fake_lock(&self) -> LockResult
    {
        // SRWLOCK doesn't provide a non-modifying check, so we assume it always
        // unlocked. It's much better and doesn't violate invariant "no
        // state change"
        LockResult::Ok(LockStatus::Done)
    }

    fn free(&self)
    {
        unsafe {
            windows_sys::Win32::System::Threading::ReleaseSRWLockExclusive(
                &self.srwlock as *const _ as *mut _,
            );
        }
    }

    fn wake_all(&self)
    {
        // SRWLOCK handles waking automatically
    }
}

impl IShare for Os
{
    fn try_share(&self, _current_iteration: usize) -> LockResult
    {
        let result = unsafe {
            windows_sys::Win32::System::Threading::TryAcquireSRWLockShared(
                &self.srwlock as *const _ as *mut _,
            )
        };

        if result
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
            windows_sys::Win32::System::Threading::ReleaseSRWLockShared(
                &self.srwlock as *const _ as *mut _,
            );
        }
    }

    fn wake_readers(&self)
    {
        // SRWLOCK handles waking automatically
    }
}
