use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicUsize, Ordering};

type _Handle = *mut c_void;
const _FILE_ANY_ACCESS: u32 = 0;
const _FILE_DEVICE_UNKNOWN: u32 = 34;
const _METHOD_BUFFERED: u32 = 0;

/// See https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/defining-i-o-control-codes
const fn _ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

pub const IOCTL_MEMORY_INITIALIZE: u32 = _ctl_code(
    _FILE_DEVICE_UNKNOWN,
    0x800,
    _METHOD_BUFFERED,
    _FILE_ANY_ACCESS,
);
pub const IOCTL_MEMORY_CLEANUP: u32 = _ctl_code(
    _FILE_DEVICE_UNKNOWN,
    0x801,
    _METHOD_BUFFERED,
    _FILE_ANY_ACCESS,
);

#[repr(C)]
pub struct MemoryInitialize {
    pub section: _Handle,
    pub event: _Handle,
    pub view_size: u64,
}

#[repr(C)]
pub struct SharedMemory {
    pub read: AtomicUsize,
    pub write: AtomicUsize,
    pub buffer: [u8; 4096],
}

impl SharedMemory {
    pub fn read(&self) -> Vec<u8> {
        let read = self.read.load(Ordering::Acquire);
        let write = self.write.load(Ordering::Acquire);
        let cap = self.buffer.len();

        // Empty when read == write
        if read == write {
            return Vec::new();
        }

        let result = if read < write {
            // Contiguous region
            self.buffer[read..write].to_vec()
        } else {
            // Wrapped region: [read..cap) + [0..write)
            let mut out = Vec::with_capacity((cap - read) + write);
            out.extend_from_slice(&self.buffer[read..cap]);
            out.extend_from_slice(&self.buffer[..write]);
            out
        };

        // Consume everything we saw
        self.read.store(write, Ordering::Release);
        result
    }

    pub fn write(&mut self, data: &[u8]) {
        let mut write = self.write.load(Ordering::Acquire);
        let read = self.read.load(Ordering::Acquire);
        let cap = self.buffer.len();

        for &byte in data {
            // Next position if we write one byte
            let next = (write + 1) % cap;
            // Leave one byte empty to distinguish full vs empty
            if next == read {
                break; // buffer full
            }

            self.buffer[write] = byte;
            write = next;
        }

        self.write.store(write, Ordering::Release);
    }
}
