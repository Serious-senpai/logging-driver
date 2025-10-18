extern crate alloc;

use core::ffi::CStr;
use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use core::{fmt, slice};

use alloc::vec;
use alloc::vec::Vec;
use wdk_sys::ntddk::{RtlAnsiStringToUnicodeString, RtlInitAnsiString};
use wdk_sys::{NT_SUCCESS, PASSIVE_LEVEL, PCUNICODE_STRING, STRING, UNICODE_STRING};

use crate::displayer::ForeignDisplayer;
use crate::error::RuntimeError;
use crate::wrappers::irql::irql_requires;
use crate::wrappers::lifetime::Lifetime;

pub struct UnicodeString {
    _native: UNICODE_STRING,
    _buffer: Vec<u16>,
}

impl<'a> UnicodeString {
    pub fn native(&'a self) -> Lifetime<'a, UNICODE_STRING> {
        Lifetime::new(self._native)
    }

    /// Clone a Unicode string from a raw pointer to a `UNICODE_STRING` structure.
    ///
    /// # Safety
    /// The pointer must point to a valid `UNICODE_STRING` structure or be null.
    pub unsafe fn from_raw(value: PCUNICODE_STRING) -> Result<Self, RuntimeError> {
        let new = match unsafe { value.as_ref() } {
            Some(s) => {
                let buf = unsafe { slice::from_raw_parts(s.Buffer, usize::from(s.Length / 2)) };
                let mut buf = buf.to_vec();
                buf.push(0);
                buf
            }
            None => vec![0],
        };

        let bytes_count = 2 * u16::try_from(new.len())?;
        Ok(Self {
            _native: UNICODE_STRING {
                Length: bytes_count - 2,
                MaximumLength: bytes_count,
                Buffer: new.as_ptr() as *mut u16,
            },
            _buffer: new,
        })
    }
}

impl TryFrom<&CStr> for UnicodeString {
    type Error = RuntimeError;

    /// Convert a C-style ANSI string to a UTF-16 string (obviously a clone is performed).
    fn try_from(value: &CStr) -> Result<Self, Self::Error> {
        irql_requires(PASSIVE_LEVEL)?;

        let mut buffer = vec![0; value.to_bytes_with_nul().len()];
        let mut native = UNICODE_STRING {
            Length: 0,
            MaximumLength: 2 * u16::try_from(buffer.len())?,
            Buffer: buffer.as_mut_ptr(),
        };

        let mut string = AnsiString::from(value);
        let status = unsafe {
            // `RtlAnsiStringToUnicodeString` allocates and copies the string (it must perform a deep copy to convert to UTF-16)
            // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-rtlansistringtounicodestring
            RtlAnsiStringToUnicodeString(&mut native, &mut string._native, 0)
        };

        if !NT_SUCCESS(status) {
            return Err(RuntimeError::Failure(status));
        }

        Ok(Self {
            _native: native,
            _buffer: buffer,
        })
    }
}

impl Display for UnicodeString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&ForeignDisplayer::Unicode(&self._native), f)
    }
}

impl Debug for UnicodeString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&ForeignDisplayer::Unicode(&self._native), f)
    }
}

pub struct AnsiString<'a> {
    _native: STRING,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> AnsiString<'a> {
    pub fn native(&'a self) -> Lifetime<'a, STRING> {
        Lifetime::new(self._native)
    }
}

impl<'a> From<&'a CStr> for AnsiString<'a> {
    /// Create a native ANSI string wrapper around a C-style string. The resulting [`STRING`] only contains
    /// a pointer to the original string data - no copy is performed.
    fn from(value: &'a CStr) -> Self {
        let mut native = STRING::default();
        unsafe {
            // `RtlInitAnsiString` only copies the pointer
            // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-rtlinitansistring
            RtlInitAnsiString(&mut native, value.as_ptr())
        };
        Self {
            _native: native,
            _phantom: PhantomData,
        }
    }
}
