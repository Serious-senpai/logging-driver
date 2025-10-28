use alloc::collections::vec_deque::VecDeque;
use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::AtomicUsize;

use common::ioctl::SharedMemory;
use wdk_sys::HANDLE;
use wdk_sys::ntddk::{MmUnmapViewInSystemSpace, ObfDereferenceObject};

use crate::wrappers::mutex::SpinLock;

#[repr(C)]
pub struct MemoryMap {
    pub section: HANDLE,
    pub event: HANDLE,
    pub mapped_base: *mut SharedMemory,
    pub view_size: u64,
}

impl MemoryMap {
    pub unsafe fn initialize(
        section: HANDLE,
        event: HANDLE,
        mapped_base: *mut SharedMemory,
        view_size: u64,
    ) -> Self {
        unsafe {
            ptr::write_volatile(
                mapped_base,
                SharedMemory {
                    read: AtomicUsize::new(0),
                    write: AtomicUsize::new(0),
                    buffer: [0; 4096],
                },
            );
        }

        Self {
            section,
            event,
            mapped_base,
            view_size,
        }
    }
}

impl Drop for MemoryMap {
    fn drop(&mut self) {
        unsafe {
            let _ = MmUnmapViewInSystemSpace(self.mapped_base as *mut c_void);
            ObfDereferenceObject(self.event);
            ObfDereferenceObject(self.section);
        }
    }
}

#[repr(C)]
pub struct DeviceState {
    pub queue: VecDeque<u8>,
    pub memmap: Option<MemoryMap>,
}

#[repr(C)]
pub struct DeviceExtension {
    pub inner: SpinLock<DeviceState>,
}
