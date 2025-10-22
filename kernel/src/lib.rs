#![no_std]

extern crate alloc;

mod displayer;
mod error;
mod log;
mod wrappers;

// #[cfg(not(test))]
extern crate wdk_panic;

// #[cfg(not(test))]
use wdk_alloc::WdkAllocator;
use wdk_sys::{
    NTSTATUS, PCUNICODE_STRING, PDEVICE_OBJECT, PDRIVER_OBJECT, PIRP, STATUS_INVALID_PARAMETER,
    STATUS_SUCCESS, STATUS_UNSUCCESSFUL,
};

use crate::error::RuntimeError;
use crate::wrappers::strings::UnicodeString;

// #[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

/// # Safety
/// Must be called by the OS.
unsafe extern "C" fn driver_unload(driver: PDRIVER_OBJECT) {
    let driver = match unsafe { driver.as_mut() } {
        Some(d) => d,
        None => {
            log!("driver_unload: PDRIVER_OBJECT is null");
            return;
        }
    };

    if let Err(e) = _handler::driver_unload(driver) {
        log!("Error when unloading driver: {e}");
    }
}

/// # Safety
/// Must be called by the OS.
unsafe extern "C" fn irp_handler(device: PDEVICE_OBJECT, irp: PIRP) -> NTSTATUS {
    let device = match unsafe { device.as_mut() } {
        Some(d) => d,
        None => {
            log!("irp_handler: PDEVICE_OBJECT is null");
            return STATUS_INVALID_PARAMETER;
        }
    };

    let irp = match unsafe { irp.as_mut() } {
        Some(i) => i,
        None => {
            log!("irp_handler: PIRP is null");
            return STATUS_INVALID_PARAMETER;
        }
    };

    match _handler::irp_handler(device, irp) {
        Ok(()) => STATUS_SUCCESS,
        Err(e) => {
            log!("Error when handling IRP: {e}");
            match e {
                RuntimeError::Failure(status) => status,
                _ => STATUS_UNSUCCESSFUL,
            }
        }
    }
}

/// # Safety
/// Must be called by the OS.
#[unsafe(export_name = "DriverEntry")]
pub unsafe extern "C" fn driver_entry(
    driver: PDRIVER_OBJECT,
    registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    let driver = match unsafe { driver.as_mut() } {
        Some(d) => d,
        None => {
            log!("driver_entry: PDRIVER_OBJECT is null");
            return STATUS_INVALID_PARAMETER;
        }
    };

    let registry_path = match unsafe { UnicodeString::from_raw(registry_path) } {
        Ok(r) => r,
        Err(e) => {
            log!("driver_entry: failed to parse registry path: {e}");
            return STATUS_INVALID_PARAMETER;
        }
    };

    match _handler::driver_entry(driver, registry_path) {
        Ok(()) => STATUS_SUCCESS,
        Err(e) => {
            log!("Error when loading driver: {e}");
            match e {
                RuntimeError::Failure(status) => status,
                _ => STATUS_UNSUCCESSFUL,
            }
        }
    }
}

mod _handler {
    use alloc::collections::VecDeque;
    use alloc::vec::Vec;
    use core::ffi::CStr;
    use core::ptr::{copy_nonoverlapping, drop_in_place, null_mut, write};
    use core::slice;

    use wdk_sys::ntddk::{IoCreateDevice, IoDeleteDevice, IofCompleteRequest};
    use wdk_sys::{
        DEVICE_OBJECT, DO_BUFFERED_IO, DO_DEVICE_INITIALIZING, DRIVER_OBJECT,
        FILE_DEVICE_SECURE_OPEN, FILE_DEVICE_UNKNOWN, IO_NO_INCREMENT, IRP, IRP_MJ_CLEANUP,
        IRP_MJ_CLOSE, IRP_MJ_CREATE, IRP_MJ_READ, IRP_MJ_WRITE, NT_SUCCESS,
        STATUS_INVALID_DEVICE_REQUEST, STATUS_INVALID_PARAMETER, STATUS_SUCCESS,
    };

    use crate::displayer::ForeignDisplayer;
    use crate::error::RuntimeError;
    use crate::log;
    use crate::wrappers::bindings::IoGetCurrentIrpStackLocation;
    use crate::wrappers::safety::{create_symbolic_link, delete_symbolic_link};
    use crate::wrappers::strings::UnicodeString;

    const DOS_NAME: &CStr = c"\\DosDevices\\LogDrvDev";
    const DEVICE_NAME: &CStr = c"\\Device\\LogDrvDev";
    const QUEUE_CAPACITY: usize = 1024;

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

    pub fn driver_entry(
        driver: &mut DRIVER_OBJECT,
        registry_path: UnicodeString,
    ) -> Result<(), RuntimeError> {
        driver.DriverUnload = Some(super::driver_unload);
        for handler in driver.MajorFunction.iter_mut() {
            *handler = Some(super::irp_handler);
        }

        log!(
            "driver_entry {:?}, registry_path={registry_path:?}",
            ForeignDisplayer::Unicode(&driver.DriverName),
        );

        let device_name = UnicodeString::try_from(DEVICE_NAME)?;

        let mut device = null_mut();
        let status = unsafe {
            let mut device_name = device_name.native().into_inner();
            IoCreateDevice(
                driver,
                size_of::<DeviceExtension>().try_into()?,
                &mut device_name,
                FILE_DEVICE_UNKNOWN,
                FILE_DEVICE_SECURE_OPEN,
                0,
                &mut device,
            )
        };
        if !NT_SUCCESS(status) {
            log!("Failed to create device: {status}");
            return Err(RuntimeError::Failure(status));
        }

        if let Some(device) = unsafe { device.as_mut() } {
            device.Flags |= DO_BUFFERED_IO;
            device.Flags &= !DO_DEVICE_INITIALIZING;

            unsafe {
                write(
                    device.DeviceExtension as *mut DeviceExtension,
                    DeviceExtension {
                        buffer: VecDeque::with_capacity(QUEUE_CAPACITY),
                    },
                );
            }
        }

        create_symbolic_link(&DOS_NAME.try_into()?, &DEVICE_NAME.try_into()?).inspect_err(|e| {
            log!("Failed to create symbolic link: {e}");
            delete_device(driver);
        })?;

        Ok(())
    }

    pub fn driver_unload(driver: &mut DRIVER_OBJECT) -> Result<(), RuntimeError> {
        log!(
            "driver_unload {:?}",
            ForeignDisplayer::Unicode(&driver.DriverName),
        );
        delete_device(driver);
        Ok(())
    }

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
                            let requested =
                                usize::try_from(unsafe { stack.Parameters.Read }.Length)?
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
                            let requested =
                                usize::try_from(unsafe { stack.Parameters.Write }.Length)?;

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
}
