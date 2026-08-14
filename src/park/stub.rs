use crate::IPark;

/// .
#[derive(Default, Debug)]
pub struct Stub;

impl Stub
{
    /// .
    pub const fn new() -> Self
    {
        Self
    }
}

unsafe impl IPark for Stub
{
    fn park(&self) {}

    fn free(&self) {}
}
