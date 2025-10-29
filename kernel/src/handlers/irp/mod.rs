mod ioctl;

use alloc::vec::Vec;
use core::ptr;

use wdk_sys::ntddk::IofCompleteRequest;
use wdk_sys::{
    DEVICE_OBJECT, IO_NO_INCREMENT, IRP, IRP_MJ_CLEANUP, IRP_MJ_CLOSE, IRP_MJ_CREATE,
    IRP_MJ_DEVICE_CONTROL, IRP_MJ_READ, NT_SUCCESS, STATUS_INVALID_DEVICE_REQUEST,
    STATUS_INVALID_PARAMETER, STATUS_SUCCESS,
};

use crate::error::RuntimeError;
use crate::handlers::DeviceExtension;
use crate::handlers::irp::ioctl::IoctlHandler;
use crate::handlers::irp::ioctl::memory_cleanup::MemoryCleanupHandler;
use crate::handlers::irp::ioctl::memory_init::MemoryInitializeHandler;
use crate::log;
use crate::wrappers::bindings::IoGetCurrentIrpStackLocation;

macro_rules! _ioctl_handle {
    ($device:expr, $irp:expr, $irpsp:expr, $input_buffer_length:expr, $($Handler:tt,)*) => {
        match unsafe { $irpsp.Parameters.DeviceIoControl.IoControlCode } {
            $($Handler::CODE => $Handler::handle($device, $irp, $irpsp, $input_buffer_length),)*
            _ => STATUS_INVALID_DEVICE_REQUEST,
        }
    };
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
                IRP_MJ_DEVICE_CONTROL => _ioctl_handle!(
                    device,
                    irp,
                    irpsp,
                    unsafe { irpsp.Parameters.DeviceIoControl.InputBufferLength },
                    MemoryInitializeHandler,
                    MemoryCleanupHandler,
                ),
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
