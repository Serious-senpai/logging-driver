extern crate alloc;

use core::ffi::CStr;
use core::marker::PhantomData;
use core::{fmt, slice};

use alloc::string::String;
use wdk_sys::ntddk::{RtlAnsiStringToUnicodeString, RtlFreeUnicodeString, RtlInitAnsiString};
use wdk_sys::{NT_SUCCESS, PASSIVE_LEVEL, PSTRING, PUNICODE_STRING, STRING, UNICODE_STRING};

use crate::error::RuntimeError;
use crate::wrappers::irql::irql_requires;

pub struct UnicodeString {
    _native: UNICODE_STRING,
    _drop: bool,
}

impl UnicodeString {
    pub fn native(&self) -> &UNICODE_STRING {
        &self._native
    }

    /// # Safety
    /// This method must only be used to pass [`PUNICODE_STRING`] to native API calls. These API
    /// calls must not mutate the underlying data in anyway.
    pub unsafe fn native_mut_ptr(&mut self) -> PUNICODE_STRING {
        &mut self._native
    }
}

impl TryFrom<&CStr> for UnicodeString {
    type Error = RuntimeError;

    fn try_from(value: &CStr) -> Result<Self, Self::Error> {
        irql_requires(PASSIVE_LEVEL)?;

        let mut native = UNICODE_STRING::default();
        let mut string = AnsiString::from(value);
        let status = unsafe {
            // `RtlAnsiStringToUnicodeString` allocates and copies the string (it must perform a deep copy to convert to UTF-16)
            // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-rtlansistringtounicodestring
            RtlAnsiStringToUnicodeString(&mut native, string.native_mut_ptr(), 1)
        };

        if !NT_SUCCESS(status) {
            return Err(RuntimeError::Failure(status));
        }

        Ok(Self {
            _native: native,
            _drop: true,
        })
    }
}

impl Drop for UnicodeString {
    fn drop(&mut self) {
        if self._drop {
            unsafe {
                RtlFreeUnicodeString(&mut self._native);
            }
        }
    }
}

pub struct AnsiString<'a> {
    _native: STRING,
    _phantom: PhantomData<&'a ()>,
}

impl AnsiString<'_> {
    pub fn native(&self) -> &STRING {
        &self._native
    }

    /// # Safety
    /// This method must only be used to pass [`PSTRING`] to native API calls. These API
    /// calls must not mutate the underlying data in anyway.
    pub unsafe fn native_mut_ptr(&mut self) -> PSTRING {
        &mut self._native
    }
}

impl<'a> From<&'a CStr> for AnsiString<'a> {
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

impl fmt::Display for UnicodeString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = String::from_utf16_lossy(unsafe {
            slice::from_raw_parts(self._native.Buffer, usize::from(self._native.Length))
        });
        write!(f, "{string}")
    }
}
