//! A filesystem‑based lock using `flock(2)` on Unix.
//!
//! This lock is useful for synchronising access to resources across processes
//! or for using a file as a coordination point. It uses an advisory lock on
//! an open file descriptor.

use std::fs::File;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};

use crate::traits::LockPolicy;
use crate::{LockResult, LockStatus};

/// A lock policy that uses `flock` on a file.
///
/// The file is opened on creation and the file descriptor is held until the
/// lock is dropped. All threads share the same file descriptor, and the kernel
/// manages the lock state.
pub struct Fs
{
    fd:   RawFd,
    path: PathBuf, // for debugging / logging
}

impl core::fmt::Debug for Fs
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        f.write_str(&format!(
            "Flock at {}",
            self.path
                .clone()
                .into_string()
                .unwrap_or_else(|path| format!("{:?}", path))
        ))
    }
}

impl Fs
{
    /// Opens or creates a file at `path` and prepares it for locking.
    ///
    /// The file is opened with read‑write access and created if it does not
    /// exist.
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self>
    {
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.as_ref())?;
        let fd = file.as_raw_fd();
        // Forget the `File` so we keep the fd alive.
        std::mem::forget(file);
        Ok(Self {
            fd,
            path: path.as_ref().to_path_buf(),
        })
    }

    /// Default path used by `Default` implementation.
    const PATH_PREFIX: &'static str = "/tmp/resync-flock";
}

impl Default for Fs
{
    fn default() -> Self
    {
        Self::new(Self::PATH_PREFIX.to_string() + &crate::util::random_hex_16())
            .expect("failed to open default lock file")
    }
}

impl Drop for Fs
{
    fn drop(&mut self)
    {
        // Close the file descriptor.
        unsafe { libc::close(self.fd) };
    }
}

// SAFETY:
// The file descriptor is an integer; the kernel handles all
// synchronisation. The fd is only closed on drop, which happens when the
// policy is destroyed.
unsafe impl Send for Fs {}
unsafe impl Sync for Fs {}

unsafe impl LockPolicy for Fs
{
    type Error = io::Error;

    type Meta = ();

    unsafe fn try_lock(
        &self,
        _current_iteration: usize,
    ) -> LockResult<Self::Meta, Self::Error>
    {
        let ret =
            unsafe { libc::flock(self.fd, libc::LOCK_EX | libc::LOCK_NB) };
        if ret == 0
        {
            Ok(LockStatus::Done(()))
        }
        else
        {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock
            {
                Ok(LockStatus::Fail)
            }
            else
            {
                Err(err)
            }
        }
    }

    unsafe fn free(&self, _: &Self::Meta)
    {
        let _ = unsafe { libc::flock(self.fd, libc::LOCK_UN) };
    }
}
