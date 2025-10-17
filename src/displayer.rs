use core::fmt::{Display, Write};
use core::{fmt, slice};
use wdk_sys::UNICODE_STRING;

pub enum ForeignDisplayer<'a> {
    Unicode(&'a UNICODE_STRING),
}

impl Display for ForeignDisplayer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ForeignDisplayer::Unicode(s) => {
                let buffer = unsafe { slice::from_raw_parts(s.Buffer, usize::from(s.Length) - 1) };
                for c in char::decode_utf16(buffer.iter().copied()) {
                    match c {
                        Ok(c) => f.write_char(c)?,
                        Err(_) => f.write_char('\u{FFFD}')?,
                    }
                }
                Ok(())
            }
        }
    }
}

impl fmt::Debug for ForeignDisplayer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;
        Display::fmt(&self, f)?;
        f.write_char('"')?;
        Ok(())
    }
}
