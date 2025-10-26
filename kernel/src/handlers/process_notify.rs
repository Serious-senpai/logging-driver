use core::sync::atomic::Ordering;

use common::types::Process;
use wdk_sys::{BOOLEAN, HANDLE};

use crate::config::{DRIVER, QUEUE_CAPACITY};
use crate::handlers::DeviceExtension;
use crate::log;

pub fn process_notify(parent_id: HANDLE, process_id: HANDLE, create: BOOLEAN) {
    let parent_id = parent_id as usize;
    let process_id = process_id as usize;
    let driver = DRIVER.load(Ordering::SeqCst);
    if let Some(driver) = unsafe { driver.as_mut() }
        && let Some(device) = unsafe { driver.DeviceObject.as_mut() }
        && let Some(queue) = unsafe {
            let extension = device.DeviceExtension as *mut DeviceExtension;
            extension.as_mut().map(|e| &mut e.buffer)
        }
    {
        let process = Process {
            parent_id,
            process_id,
            create: create != 0,
        };

        match serde_json::to_vec(&process) {
            Ok(data) => {
                queue.extend(data);
                queue.push_back(b'\n');
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
