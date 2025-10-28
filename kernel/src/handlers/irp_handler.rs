use alloc::vec::Vec;
use core::ptr;

use common::ioctl::{
    IOCTL_MEMORY_CLEANUP, IOCTL_MEMORY_INITIALIZE, MemoryInitialize, SharedMemory,
};
use wdk_sys::_MODE::UserMode;
use wdk_sys::ntddk::{
    IofCompleteRequest, MmMapViewInSystemSpace, ObReferenceObjectByHandle, ObfDereferenceObject,
};
use wdk_sys::{
    DEVICE_OBJECT, EVENT_MODIFY_STATE, ExEventObjectType, IO_NO_INCREMENT, IO_STACK_LOCATION, IRP,
    IRP_MJ_CLEANUP, IRP_MJ_CLOSE, IRP_MJ_CREATE, IRP_MJ_DEVICE_CONTROL, IRP_MJ_READ, NT_SUCCESS,
    NTSTATUS, SECTION_MAP_READ, SECTION_MAP_WRITE, STATUS_DEVICE_BUSY,
    STATUS_INVALID_DEVICE_REQUEST, STATUS_INVALID_PARAMETER, STATUS_SUCCESS, SYNCHRONIZE,
};

use crate::error::RuntimeError;
use crate::handlers::DeviceExtension;
use crate::log;
use crate::state::MemoryMap;
use crate::wrappers::bindings::IoGetCurrentIrpStackLocation;

fn _ioctl_handler(
    device: &mut DEVICE_OBJECT,
    irp: &mut IRP,
    irpsp: &IO_STACK_LOCATION,
) -> NTSTATUS {
    match unsafe { irpsp.Parameters.DeviceIoControl.IoControlCode } {
        IOCTL_MEMORY_INITIALIZE => {
            // IMPORTANT: Do not hold the spinlock while calling PASSIVE_LEVEL-only APIs.
            // Validate input and perform handle referencing and mapping at PASSIVE_LEVEL first.
            let extension = device.DeviceExtension as *mut DeviceExtension;
            match unsafe { extension.as_mut() } {
                Some(extension_ref) => {
                    if unsafe { irpsp.Parameters.DeviceIoControl.InputBufferLength }
                        != size_of::<MemoryInitialize>() as u32
                    {
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
                    let status = unsafe {
                        MmMapViewInSystemSpace(section, &mut mapped_base, &mut view_size)
                    };
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
        IOCTL_MEMORY_CLEANUP => {
            // Take the mapping out under the lock, but ensure the actual drop occurs after releasing the lock.
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
        _ => STATUS_INVALID_DEVICE_REQUEST,
    }
}

pub fn irp_handler(device: &mut DEVICE_OBJECT, irp: &mut IRP) -> Result<(), RuntimeError> {
    irp.IoStatus.Information = 0;
    let status = match unsafe { IoGetCurrentIrpStackLocation(irp).as_ref() } {
        Some(irpsp) => {
            log!("Received IRP {}", irpsp.MajorFunction);

            match irpsp.MajorFunction.into() {
                IRP_MJ_CREATE | IRP_MJ_CLOSE | IRP_MJ_CLEANUP => STATUS_SUCCESS,
                IRP_MJ_READ => {
                    let inner = unsafe {
                        let extension = device.DeviceExtension as *mut DeviceExtension;
                        extension.as_mut().map(|e| &mut e.inner)
                    };
                    if let Some(inner) = inner {
                        let mut inner = inner.acquire();
                        let requested = usize::try_from(unsafe { irpsp.Parameters.Read.Length })?
                            .min(inner.queue.len());

                        let src = inner.queue.drain(..requested).collect::<Vec<u8>>();
                        unsafe {
                            let dst = irp.AssociatedIrp.SystemBuffer as *mut u8;
                            ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
                        }

                        irp.IoStatus.Information = src.len().try_into()?;
                    }

                    STATUS_SUCCESS
                }
                IRP_MJ_DEVICE_CONTROL => _ioctl_handler(device, irp, irpsp),
                _ => STATUS_INVALID_DEVICE_REQUEST,
            }
        }
        None => {
            log!("Received unknown IRP");
            STATUS_INVALID_PARAMETER
        }
    };

    irp.IoStatus.__bindgen_anon_1.Status = status;
    unsafe {
        IofCompleteRequest(irp, IO_NO_INCREMENT.try_into()?);
    }

    if NT_SUCCESS(status) {
        Ok(())
    } else {
        Err(RuntimeError::Failure(status))
    }
}
