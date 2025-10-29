use common::ioctl::IOCTL_MEMORY_CLEANUP;
use wdk_sys::{NTSTATUS, STATUS_DEVICE_BUSY, STATUS_SUCCESS};

use crate::handlers::DeviceExtension;
use crate::handlers::irp::ioctl::IoctlHandler;

pub struct MemoryCleanupHandler;

impl IoctlHandler for MemoryCleanupHandler {
    const CODE: u32 = IOCTL_MEMORY_CLEANUP;

    fn handle(
        device: &mut wdk_sys::DEVICE_OBJECT,
        _: &mut wdk_sys::IRP,
        _: &wdk_sys::IO_STACK_LOCATION,
        _: u32,
    ) -> NTSTATUS {
        let extension = device.DeviceExtension as *mut DeviceExtension;
        match unsafe { extension.as_mut() } {
            Some(extension_ref) => {
                let old = {
                    let mut inner = extension_ref.inner.acquire();
                    inner.memmap.take()
                };
                drop(old);
                STATUS_SUCCESS
            }
            None => STATUS_DEVICE_BUSY,
        }
    }
}
