mod driver_entry;
mod driver_unload;
mod irp_handler;
mod process_notify;

use alloc::collections::vec_deque::VecDeque;
use core::ptr::drop_in_place;
use wdk_sys::ntddk::IoDeleteDevice;

use wdk_sys::DRIVER_OBJECT;

pub use driver_entry::driver_entry;
pub use driver_unload::driver_unload;
pub use irp_handler::irp_handler;
pub use process_notify::process_notify;

use crate::config::DOS_NAME;
use crate::log;
use crate::wrappers::safety::delete_symbolic_link;

#[repr(C)]
pub struct DeviceExtension {
    pub buffer: VecDeque<u8>,
}

pub fn delete_device(driver: &DRIVER_OBJECT) {
    match DOS_NAME.try_into() {
        Ok(dos_name) => {
            if let Err(e) = delete_symbolic_link(&dos_name) {
                log!("Failed to remove symlink: {e}");
            }
        }
        Err(e) => {
            log!("Cannot convert {DOS_NAME:?} to UnicodeString: {e}");
        }
    }

    let device = driver.DeviceObject;
    if let Some(device) = unsafe { device.as_mut() } {
        unsafe {
            drop_in_place(device.DeviceExtension as *mut DeviceExtension);
            IoDeleteDevice(device);
        }
    }
}
