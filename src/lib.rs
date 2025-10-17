#![no_std]

pub mod display;
pub mod error;
pub mod log;
pub mod wrappers;

extern crate alloc;
#[cfg(not(test))]
extern crate wdk_panic;

use core::{ffi::CStr, ptr::null_mut};

#[cfg(not(test))]
use wdk_alloc::WdkAllocator;
use wdk_sys::ntddk::{
    IoCreateDevice, IoCreateSymbolicLink, IoDeleteDevice, IoDeleteSymbolicLink, IofCompleteRequest,
};
use wdk_sys::{
    DRIVER_OBJECT, FILE_DEVICE_SECURE_OPEN, FILE_DEVICE_UNKNOWN, IO_NO_INCREMENT, IRP_MJ_READ,
    IRP_MJ_WRITE, NT_SUCCESS, NTSTATUS, PCUNICODE_STRING, PDEVICE_OBJECT, PDRIVER_DISPATCH,
    PDRIVER_OBJECT, PIRP, STATUS_INVALID_DEVICE_REQUEST, STATUS_INVALID_PARAMETER, STATUS_SUCCESS,
    STATUS_UNSUCCESSFUL,
};

use crate::display::Displayable;
use crate::error::RuntimeError;
use crate::wrappers::bindings::IoGetCurrentIrpStackLocation;
use crate::wrappers::strings::UnicodeString;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

const DOS_NAME: &CStr = c"\\DosDevices\\LogDrvDev";
const DEVICE_NAME: &CStr = c"\\Device\\LogDrvDev";

fn _delete_device(driver: &DRIVER_OBJECT) -> Result<(), RuntimeError> {
    let dos_name = UnicodeString::try_from(DOS_NAME)?;

    let status = unsafe { IoDeleteSymbolicLink(&mut dos_name.native().into_inner()) };
    if !NT_SUCCESS(status) {
        log!("Failed to remove symlink: {status}");
    }

    let device = driver.DeviceObject;
    if !device.is_null() {
        unsafe {
            IoDeleteDevice(device);
        }
    }

    Ok(())
}

fn _set_driver_callback(driver: &mut DRIVER_OBJECT, irp_code: u32, callback: PDRIVER_DISPATCH) {
    match usize::try_from(irp_code) {
        Ok(code) => driver.MajorFunction[code] = callback,
        Err(e) => {
            log!("Unable to set driver callback for {irp_code}: {e}");
        }
    }
}

fn _driver_entry(
    driver: &mut DRIVER_OBJECT,
    registry_path: UnicodeString,
) -> Result<(), RuntimeError> {
    driver.DriverUnload = Some(driver_unload);
    for handler in driver.MajorFunction.iter_mut() {
        *handler = Some(_irp_handler);
    }

    log!(
        "driver_entry {:?}, registry_path={registry_path}",
        driver.DriverName.display(),
    );

    let dos_name = UnicodeString::try_from(DOS_NAME)?;
    let device_name = UnicodeString::try_from(DEVICE_NAME)?;

    let mut device = null_mut();
    let status = unsafe {
        IoCreateDevice(
            driver,
            0,
            &mut device_name.native().into_inner(),
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

    let status = unsafe {
        IoCreateSymbolicLink(
            &mut dos_name.native().into_inner(),
            &mut device_name.native().into_inner(),
        )
    };
    if !NT_SUCCESS(status) {
        log!("Failed to create symbolic link: {status}");
        _delete_device(driver)?;
        return Err(RuntimeError::Failure(status));
    }

    Ok(())
}

fn _driver_unload(driver: &mut DRIVER_OBJECT) -> Result<(), RuntimeError> {
    log!("driver_unload {:?}", driver.DriverName.display());
    _delete_device(driver)?;
    Ok(())
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

    match _driver_entry(driver, registry_path) {
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

    if let Err(e) = _driver_unload(driver) {
        log!("Error when unloading driver: {e}");
    }
}

/// # Safety
/// Must be called by the OS.
unsafe extern "C" fn _irp_handler(_: PDEVICE_OBJECT, irp: PIRP) -> NTSTATUS {
    let irp = match unsafe { irp.as_mut() } {
        Some(i) => i,
        None => {
            log!("irp_handler: PIRP is null");
            return STATUS_INVALID_PARAMETER;
        }
    };

    let status = match unsafe { IoGetCurrentIrpStackLocation(irp).as_ref() } {
        Some(stack) => {
            log!("Received IRP {}", stack.MajorFunction);
            match stack.MajorFunction.into() {
                IRP_MJ_READ | IRP_MJ_WRITE => STATUS_SUCCESS,
                _ => STATUS_INVALID_DEVICE_REQUEST,
            }
        }
        None => {
            log!("Received unknown IRP");
            STATUS_INVALID_PARAMETER
        }
    };

    irp.IoStatus.__bindgen_anon_1.Status = status;
    irp.IoStatus.Information = 0;
    unsafe {
        IofCompleteRequest(
            irp,
            IO_NO_INCREMENT
                .try_into()
                .expect("IO_NO_INCREMENT must fit into i8"),
        );
    }

    status
}
