#![no_std]

#[cfg(not(test))]
extern crate wdk_panic;

extern crate alloc;
use alloc::string::ToString;
mod bindings;
mod display;
mod log;
mod unicode;
use core::{ffi::CStr, ptr::null_mut};

#[cfg(not(test))]
use wdk_alloc::WdkAllocator;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

use bindings::IoGetCurrentIrpStackLocation;
use wdk_sys::ntddk::{
    IoCreateDevice, IoCreateSymbolicLink, IoDeleteDevice, IoDeleteSymbolicLink, IofCompleteRequest,
};
use wdk_sys::{
    DRIVER_OBJECT, FILE_DEVICE_SECURE_OPEN, FILE_DEVICE_UNKNOWN, IO_NO_INCREMENT, NT_SUCCESS,
    NTSTATUS, PCUNICODE_STRING, PDEVICE_OBJECT, PDRIVER_DISPATCH, PDRIVER_OBJECT, PIRP,
    STATUS_INVALID_PARAMETER, STATUS_SUCCESS,
};

use crate::display::Displayable;
use crate::unicode::UnicodeString;

const DOS_NAME: &CStr = c"\\DosDevices\\LogDrvDev";
const DEVICE_NAME: &CStr = c"\\Device\\LogDrvDev";

fn _delete_device(driver: &DRIVER_OBJECT) {
    let mut dos_name = UnicodeString::from(DOS_NAME);

    let status = unsafe { IoDeleteSymbolicLink(&mut dos_name.native) };
    if !NT_SUCCESS(status) {
        log!("Failed to remove symlink: {status}");
    }

    let device = driver.DeviceObject;
    if !device.is_null() {
        unsafe {
            IoDeleteDevice(device);
        }
    }
}

fn _set_driver_callback(driver: &mut DRIVER_OBJECT, irp_code: u32, callback: PDRIVER_DISPATCH) {
    match usize::try_from(irp_code) {
        Ok(code) => driver.MajorFunction[code] = callback,
        Err(e) => {
            log!("Unable to set driver callback for {irp_code}: {e}");
        }
    }
}

/// # Safety
/// Must be called by the OS.
#[unsafe(export_name = "DriverEntry")]
pub unsafe extern "system" fn driver_entry(
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

    driver.DriverUnload = Some(_driver_unload);
    for handler in driver.MajorFunction.iter_mut() {
        *handler = Some(_irp_handler);
    }

    let registry_path = match unsafe { registry_path.as_ref() } {
        Some(r) => r.display(),
        None => {
            log!("driver_entry: registry_path is null");
            "".to_string()
        }
    };

    log!(
        "driver_entry {:?}, registry_path={registry_path:?}",
        driver.DriverName.display(),
    );

    let mut dos_name = UnicodeString::from(DOS_NAME);
    let mut device_name = UnicodeString::from(DEVICE_NAME);

    let mut device = null_mut();
    let status = unsafe {
        IoCreateDevice(
            driver,
            0,
            &mut device_name.native,
            FILE_DEVICE_UNKNOWN,
            FILE_DEVICE_SECURE_OPEN,
            0,
            &mut device,
        )
    };
    if !NT_SUCCESS(status) {
        log!("Failed to create device: {status}");
        return status;
    }

    let status = unsafe { IoCreateSymbolicLink(&mut dos_name.native, &mut device_name.native) };
    if !NT_SUCCESS(status) {
        log!("Failed to create symbolic link: {status}");
        _delete_device(driver);
        return status;
    }

    STATUS_SUCCESS
}

/// # Safety
/// Must be called by the OS.
unsafe extern "C" fn _driver_unload(driver: PDRIVER_OBJECT) {
    let driver = match unsafe { driver.as_ref() } {
        Some(d) => d,
        None => {
            log!("driver_unload: PDRIVER_OBJECT is null");
            return;
        }
    };

    log!("driver_unload {:?}", driver.DriverName.display());

    _delete_device(driver);
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

    match unsafe { IoGetCurrentIrpStackLocation(irp).as_ref() } {
        Some(stack) => {
            log!("Received IRP {}", stack.MajorFunction);
        }
        None => {
            log!("Received unknown IRP");
        }
    }

    irp.IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS;
    irp.IoStatus.Information = 0;
    unsafe {
        IofCompleteRequest(
            irp,
            IO_NO_INCREMENT
                .try_into()
                .expect("IO_NO_INCREMENT must fit into i8"),
        );
    }

    STATUS_SUCCESS
}
