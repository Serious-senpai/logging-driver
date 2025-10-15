extern crate alloc;

use alloc::vec::Vec;
use core::ffi::CStr;

use wdk_sys::UNICODE_STRING;
use wdk_sys::ntddk::RtlInitUnicodeString;

pub struct UnicodeString {
    pub native: UNICODE_STRING,
    _buffer: Vec<u16>,
}

impl From<&CStr> for UnicodeString {
    fn from(value: &CStr) -> Self {
        let mut native = UNICODE_STRING::default();
        let buffer = value
            .to_bytes_with_nul()
            .iter()
            .map(|c| *c as u16)
            .collect::<Vec<u16>>();

        unsafe { RtlInitUnicodeString(&mut native, buffer.as_ptr()) };
        Self {
            native,
            _buffer: buffer,
        }
    }
}
