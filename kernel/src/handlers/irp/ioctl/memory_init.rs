use core::ptr;

use common::ioctl::{IOCTL_MEMORY_INITIALIZE, MemoryInitialize, SharedMemory};
use wdk_sys::_MODE::UserMode;
use wdk_sys::ntddk::{MmMapViewInSystemSpace, ObReferenceObjectByHandle, ObfDereferenceObject};
use wdk_sys::{
    EVENT_MODIFY_STATE, ExEventObjectType, NT_SUCCESS, NTSTATUS, SECTION_MAP_READ,
    SECTION_MAP_WRITE, STATUS_DEVICE_BUSY, STATUS_INVALID_PARAMETER, STATUS_SUCCESS, SYNCHRONIZE,
};

use crate::handlers::DeviceExtension;
use crate::handlers::irp::ioctl::IoctlHandler;
use crate::log;
use crate::state::MemoryMap;

pub struct MemoryInitializeHandler;

impl IoctlHandler for MemoryInitializeHandler {
    const CODE: u32 = IOCTL_MEMORY_INITIALIZE;

    fn handle(
        device: &mut wdk_sys::DEVICE_OBJECT,
        irp: &mut wdk_sys::IRP,
        _: &wdk_sys::IO_STACK_LOCATION,
        input_buffer_length: u32,
    ) -> NTSTATUS {
        let extension = device.DeviceExtension as *mut DeviceExtension;
        match unsafe { extension.as_mut() } {
            Some(extension_ref) => {
                if input_buffer_length != size_of::<MemoryInitialize>() as u32 {
                    return STATUS_INVALID_PARAMETER;
                }

                let input = match unsafe {
                    let ptr = irp.AssociatedIrp.SystemBuffer as *const MemoryInitialize;
                    ptr.as_ref()
                } {
                    Some(input) => input,
                    None => return STATUS_INVALID_PARAMETER,
                };

                if input.view_size < size_of::<SharedMemory>() as u64 {
                    return STATUS_INVALID_PARAMETER;
                }

                log!("Received memory with view size {}", input.view_size);

                // Reference the section and event handles from user mode.
                let mut section = ptr::null_mut();
                let status = unsafe {
                    ObReferenceObjectByHandle(
                        input.section,
                        SECTION_MAP_READ | SECTION_MAP_WRITE,
                        ptr::null_mut(),
                        UserMode as i8,
                        &mut section,
                        ptr::null_mut(),
                    )
                };
                if !NT_SUCCESS(status) {
                    return status;
                }

                let mut event = ptr::null_mut();
                let status = unsafe {
                    ObReferenceObjectByHandle(
                        input.event,
                        EVENT_MODIFY_STATE | SYNCHRONIZE,
                        *ExEventObjectType,
                        UserMode as i8,
                        &mut event,
                        ptr::null_mut(),
                    )
                };
                if !NT_SUCCESS(status) {
                    unsafe {
                        ObfDereferenceObject(section);
                    }
                    return status;
                }

                // Map the section into system space.
                let mut mapped_base = ptr::null_mut();
                let mut view_size = input.view_size;
                let status =
                    unsafe { MmMapViewInSystemSpace(section, &mut mapped_base, &mut view_size) };
                if !NT_SUCCESS(status) {
                    unsafe {
                        ObfDereferenceObject(event);
                        ObfDereferenceObject(section);
                    }
                    return status;
                }

                // Build the new mapping object.
                let new_map = unsafe {
                    MemoryMap::initialize(
                        section,
                        event,
                        mapped_base as *mut SharedMemory,
                        view_size,
                    )
                };

                // Swap it into place under the spinlock, but drop the old mapping AFTER releasing the lock.
                let old = {
                    let mut inner = extension_ref.inner.acquire();
                    inner.memmap.replace(new_map)
                };

                // Now that the lock is released (IRQL restored), drop the old mapping safely at PASSIVE_LEVEL.
                drop(old);

                STATUS_SUCCESS
            }
            None => STATUS_DEVICE_BUSY,
        }
    }
}
