use core::sync::atomic::Ordering;

use common::types::Event;
use wdk_sys::ntddk::KeSetEvent;
use wdk_sys::{BOOLEAN, HANDLE};

use crate::config::{DRIVER, QUEUE_CAPACITY};
use crate::log;
use crate::state::DeviceExtension;

/// # Safety
/// Must be called by the OS.
pub unsafe extern "C" fn thread_notify(process_id: HANDLE, thread_id: HANDLE, create: BOOLEAN) {
    let process_id = process_id as usize;
    let thread_id = thread_id as usize;
    let driver = DRIVER.load(Ordering::SeqCst);
    if let Some(driver) = unsafe { driver.as_mut() }
        && let Some(device) = unsafe { driver.DeviceObject.as_mut() }
        && let Some(inner) = unsafe {
            let extension = device.DeviceExtension as *mut DeviceExtension;
            extension.as_mut().map(|e| &mut e.inner)
        }
    {
        let event = Event::Thread {
            process_id,
            thread_id,
            create: create != 0,
        };

        match postcard::to_allocvec_cobs(&event) {
            Ok(data) => {
                let mut inner = inner.acquire();
                inner.queue.extend(&data);
                while inner.queue.len() > QUEUE_CAPACITY {
                    inner.queue.pop_front();
                }

                if let Some(memmap) = &mut inner.memmap
                    && let Some(memory) = unsafe { memmap.mapped_base.as_mut() }
                {
                    memory.write(&data);
                    unsafe {
                        KeSetEvent(memmap.event as *mut _, 0, 0);
                    }
                }
            }
            Err(e) => {
                log!("Failed to serialize thread: {e}");
            }
        }
    }
}
