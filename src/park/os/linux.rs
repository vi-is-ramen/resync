use core::sync::atomic::AtomicU32;

use crate::IPark;

/// .
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Os(AtomicU32);

impl Os
{
    /// .
    pub const fn new() -> Self
    {
        Self(AtomicU32::new(0))
    }

    #[inline]
    fn futex_wait(&self, expected: u32)
    {
        let ptr = self.0.as_ptr();
        unsafe {
            libc::syscall(
                libc::SYS_futex,
                ptr,
                libc::FUTEX_WAIT,
                expected,
                std::ptr::null::<libc::timespec>(),
            );
        }
    }

    #[inline]
    fn futex_wake(&self)
    {
        let ptr = self.0.as_ptr();
        unsafe {
            libc::syscall(
                libc::SYS_futex,
                ptr,
                libc::FUTEX_WAKE,
                1, // wake one waiter
            );
        }
    }
}

unsafe impl IPark for Os
{
    fn park(&self)
    {
        self.futex_wait(0);
    }
    fn free(&self)
    {
        self.futex_wake();
    }
}
