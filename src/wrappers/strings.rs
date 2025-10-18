extern crate alloc;

use core::ffi::CStr;
use core::fmt::{Debug, Display};
use core::{fmt, slice};

use alloc::vec;
use alloc::vec::Vec;
use wdk_sys::ntddk::{RtlAnsiStringToUnicodeString, RtlInitAnsiString};
use wdk_sys::{NT_SUCCESS, PASSIVE_LEVEL, PCUNICODE_STRING, STRING, UNICODE_STRING};

use crate::displayer::ForeignDisplayer;
use crate::error::RuntimeError;
use crate::wrappers::irql::irql_requires;
use crate::wrappers::phantom::Lifetime;

/// Wrapper around an owned [`UNICODE_STRING`] structure.
///
/// Underlying implementation **must never move the buffer semantically**.
pub struct UnicodeString {
    /// The `UNICODE_STRING.Buffer` field points to `_buffer.as_ptr()`.
    _native: UNICODE_STRING,

    /// **This buffer must never be moved semantically.** According to the
    /// [docs](https://doc.rust-lang.org/std/pin/index.html)
    /// (at the time of writing):
    ///
    /// > The second option is a viable solution to the problem for some use cases,
    /// > in particular for self-referential types. Under this model, any type that has
    /// > an address sensitive state would ultimately store its data in something like
    /// > a [`Box<T>`](https://doc.rust-lang.org/std/boxed/struct.Box.html), carefully
    /// > manage internal access to that data to ensure no moves or other invalidation
    /// > occurs, and finally provide a safe interface on top.
    ///
    /// > There are a couple of linked disadvantages to using this model. The most
    /// > significant is that each individual object must assume it is on its own to
    /// > ensure that its data does not become moved or otherwise invalidated. Since
    /// > there is no shared contract between values of different types, an object
    /// > cannot assume that others interacting with it will properly respect the
    /// > invariants around interacting with its data and must therefore protect it
    /// > from everyone. Because of this, composition of address-sensitive types
    /// > requires at least a level of pointer indirection each time a new object is
    /// > added to the mix (and, practically, a heap allocation).
    ///
    /// Note that [`Vec`] manages its buffer pointer internally, so using stuff like
    /// [`Pin<Box<Pinned<Vec<u16>>>>`](https://doc.rust-lang.org/std/pin/struct.Pin.html)
    /// will not work as expected (it pins the [`Vec`], not the internal pointer).
    _buffer: Vec<u16>,
}

impl UnicodeString {
    pub fn native(&self) -> Lifetime<'_, UNICODE_STRING> {
        Lifetime::new(self._native)
    }

    /// Clone a Unicode string from a raw pointer to a [`UNICODE_STRING`] structure.
    ///
    /// # Safety
    /// The pointer must point to a valid [`UNICODE_STRING`] structure or be null.
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

/// Wrapper around an owned [`STRING`] structure.
///
/// Underlying implementation **must refer to the notes of [`UnicodeString`]**.
pub struct AnsiString {
    _native: STRING,

    /// **Refer to the notes of [`UnicodeString`]**.
    _buffer: Vec<u8>,
}

impl AnsiString {
    pub fn native(&self) -> Lifetime<'_, STRING> {
        Lifetime::new(self._native)
    }
}

impl<'a> From<&'a CStr> for AnsiString {
    /// Convert a C-style ANSI string to a [`STRING`] structure (a clone is performed).
    fn from(value: &'a CStr) -> Self {
        let mut native = STRING::default();
        let buffer = value.to_bytes_with_nul().to_vec();

        unsafe {
            // `RtlInitAnsiString` only copies the pointer
            // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-rtlinitansistring
            RtlInitAnsiString(&mut native, buffer.as_ptr() as *const i8);
        };
        Self {
            _native: native,
            _buffer: buffer,
        }
    }
}
