use alloc::vec::Vec;
use core::ptr::copy_nonoverlapping;
use core::slice;

use wdk_sys::ntddk::IofCompleteRequest;
use wdk_sys::{
    DEVICE_OBJECT, IO_NO_INCREMENT, IRP, IRP_MJ_CLEANUP, IRP_MJ_CLOSE, IRP_MJ_CREATE, IRP_MJ_READ,
    IRP_MJ_WRITE, NT_SUCCESS, STATUS_INVALID_DEVICE_REQUEST, STATUS_INVALID_PARAMETER,
    STATUS_SUCCESS,
};

use crate::error::RuntimeError;
use crate::handlers::DeviceExtension;
use crate::log;
use crate::wrappers::bindings::IoGetCurrentIrpStackLocation;

pub fn irp_handler(device: &mut DEVICE_OBJECT, irp: &mut IRP) -> Result<(), RuntimeError> {
    irp.IoStatus.Information = 0;
    let status = match unsafe { IoGetCurrentIrpStackLocation(irp).as_ref() } {
        Some(stack) => {
            log!("Received IRP {}", stack.MajorFunction);
            match stack.MajorFunction.into() {
                IRP_MJ_CREATE | IRP_MJ_CLOSE | IRP_MJ_CLEANUP => STATUS_SUCCESS,
                IRP_MJ_READ => {
                    let queue = unsafe {
                        let extension = device.DeviceExtension as *mut DeviceExtension;
                        extension.as_mut().map(|e| &mut e.buffer)
                    };
                    if let Some(queue) = queue {
                        let requested = usize::try_from(unsafe { stack.Parameters.Read }.Length)?
                            .min(queue.len());
                        let src = queue.drain(..requested).collect::<Vec<u8>>();
                        let dst = unsafe { irp.AssociatedIrp.SystemBuffer as *mut u8 };
                        unsafe {
                            copy_nonoverlapping(src.as_ptr(), dst, src.len());
                        }

                        irp.IoStatus.Information = src.len().try_into()?;
                    }

                    STATUS_SUCCESS
                }
                IRP_MJ_WRITE => {
                    let queue = unsafe {
                        let extension = device.DeviceExtension as *mut DeviceExtension;
                        extension.as_mut().map(|e| &mut e.buffer)
                    };
                    if let Some(queue) = queue {
                        let src = unsafe { irp.AssociatedIrp.SystemBuffer as *mut u8 };
                        let requested = usize::try_from(unsafe { stack.Parameters.Write }.Length)?;

                        for byte in unsafe { slice::from_raw_parts(src, requested) } {
                            if queue.len() == queue.capacity() {
                                queue.pop_front();
                            }
                            queue.push_back(*byte);
                        }

                        irp.IoStatus.Information = requested.try_into()?;
                    }

                    STATUS_SUCCESS
                }
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
