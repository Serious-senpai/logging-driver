use core::sync::atomic::Ordering;

use common::types::Event;
use wdk_sys::{BOOLEAN, HANDLE};

use crate::config::{DRIVER, QUEUE_CAPACITY};
use crate::handlers::DeviceExtension;
use crate::log;

/// # Safety
/// Must be called by the OS.
pub unsafe extern "C" fn thread_notify(process_id: HANDLE, thread_id: HANDLE, create: BOOLEAN) {
    let process_id = process_id as usize;
    let thread_id = thread_id as usize;
    let driver = DRIVER.load(Ordering::SeqCst);
    if let Some(driver) = unsafe { driver.as_mut() }
        && let Some(device) = unsafe { driver.DeviceObject.as_mut() }
        && let Some(queue) = unsafe {
            let extension = device.DeviceExtension as *mut DeviceExtension;
            extension.as_mut().map(|e| &mut e.buffer)
        }
    {
        let process = Event::Thread {
            process_id,
            thread_id,
            create: create != 0,
        };

        match postcard::to_allocvec_cobs(&process) {
            Ok(data) => {
                let mut queue = queue.acquire();
                queue.extend(data);
                while queue.len() > QUEUE_CAPACITY {
                    queue.pop_front();
                }
            }
            Err(e) => {
                log!("Failed to serialize process: {e}");
            }
        }
    }
}
