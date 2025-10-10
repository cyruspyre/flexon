pub trait Sealed {}

pub(crate) trait Bypass {
    fn bypass<'a, 'b>(&'a mut self) -> &'b mut Self {
        unsafe { &mut *(self as *mut Self) }
    }
}

impl<T> Bypass for T {}
