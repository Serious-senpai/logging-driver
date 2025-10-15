extern crate alloc;

use alloc::string::String;
use core::slice;

use wdk_sys::UNICODE_STRING;

pub trait Displayable {
    fn display<'a>(&'a self) -> String;
}

impl Displayable for UNICODE_STRING {
    fn display<'a>(&'a self) -> String {
        let buffer = unsafe { slice::from_raw_parts(self.Buffer, usize::from(self.Length / 2)) };
        let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());

        String::from_utf16_lossy(&buffer[..end])
    }
}
