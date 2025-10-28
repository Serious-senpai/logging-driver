use std::collections::VecDeque;
use std::error::Error;
use std::ffi::c_void;
use std::fs::OpenOptions;
use std::os::windows::io::IntoRawHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{ptr, thread};

use common::ioctl::{
    IOCTL_MEMORY_CLEANUP, IOCTL_MEMORY_INITIALIZE, MemoryInitialize, SharedMemory,
};
use common::types::Event;
use tokio::signal;
use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Memory::{
    CreateFileMappingA, FILE_MAP_READ, FILE_MAP_WRITE, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
    PAGE_READWRITE, UnmapViewOfFile,
};
use windows::Win32::System::Threading::{CreateEventA, WaitForSingleObject};
use windows::core::PCSTR;

use crate::config::DEVICE_NAME;

#[derive(Debug)]
struct _MappedMemoryGuard {
    _base: MEMORY_MAPPED_VIEW_ADDRESS,
}

impl _MappedMemoryGuard {
    fn new(base: MEMORY_MAPPED_VIEW_ADDRESS) -> Self {
        Self { _base: base }
    }

    fn value(&self) -> *mut SharedMemory {
        self._base.Value as *mut SharedMemory
    }
}

impl Drop for _MappedMemoryGuard {
    fn drop(&mut self) {
        if !self.value().is_null() {
            unsafe {
                let _ = UnmapViewOfFile(self._base);
            }
        }
    }
}

struct _DeviceCleanup {
    _device: HANDLE,
}

impl Drop for _DeviceCleanup {
    fn drop(&mut self) {
        unsafe {
            let _ = DeviceIoControl(
                self._device,
                IOCTL_MEMORY_CLEANUP,
                None,
                0,
                None,
                0,
                None,
                None,
            );
        }
    }
}

fn _stream(stopped: Arc<AtomicBool>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let view_size = size_of::<SharedMemory>();
    let hmap = unsafe {
        CreateFileMappingA(
            INVALID_HANDLE_VALUE,
            None,
            PAGE_READWRITE,
            ((view_size >> 32) & 0xFFFFFFFF) as u32,
            (view_size & 0xFFFFFFFF) as u32,
            PCSTR::from_raw(ptr::null()),
        )?
    };

    let base = _MappedMemoryGuard::new(unsafe {
        MapViewOfFile(hmap, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, view_size)
    });

    if !base.value().is_null() {
        let event = unsafe { CreateEventA(None, false, false, PCSTR::from_raw(ptr::null()))? };

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(DEVICE_NAME)
            .expect("Unable to open device");
        let device = HANDLE(file.into_raw_handle());

        let setup = MemoryInitialize {
            section: hmap.0,
            event: event.0,
            view_size: view_size as u64,
        };

        unsafe {
            DeviceIoControl(
                device,
                IOCTL_MEMORY_INITIALIZE,
                Some(&setup as *const MemoryInitialize as *const c_void),
                size_of::<MemoryInitialize>() as u32,
                None,
                0,
                None,
                None,
            )?;
        }

        let cleanup = _DeviceCleanup { _device: device };

        let mut queue = VecDeque::new();
        let mut current = vec![];
        while !stopped.load(Ordering::SeqCst) {
            unsafe {
                WaitForSingleObject(event, 1000);
            }

            let data = unsafe { &*base.value() }.read();
            queue.extend(&data);
            while let Some(byte) = queue.pop_front() {
                current.push(byte);

                if byte == 0 {
                    print!("Received {} bytes: {current:?}", current.len());
                    let event = postcard::from_bytes_cobs::<Event>(&mut current);
                    current.clear();
                    println!(" -> {event:?}");
                }
            }
        }

        drop(cleanup);
        drop(base);
    }

    Ok(())
}

pub async fn stream() -> Result<(), Box<dyn Error + Send + Sync>> {
    let stopped = Arc::new(AtomicBool::new(false));
    let stopped_clone = stopped.clone();
    let thread = thread::spawn(move || {
        if let Err(e) = _stream(stopped_clone) {
            println!("Error while streaming: {e}");
        }
    });

    signal::ctrl_c().await?;
    println!("Received Ctrl-C signal.");

    stopped.store(true, Ordering::SeqCst);
    thread.join().expect("Failed to join read thread");

    Ok(())
}
