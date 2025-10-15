#![no_std]

#[cfg(not(test))]
extern crate wdk_panic;

extern crate alloc;
use alloc::string::ToString;
mod display;
mod log;
mod unicode;

use core::{ffi::CStr, ptr::null_mut};

#[cfg(not(test))]
use wdk_alloc::WdkAllocator;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

use wdk_sys::ntddk::{IoCreateDevice, IoCreateSymbolicLink, IoDeleteDevice, IoDeleteSymbolicLink};
use wdk_sys::{
    FILE_DEVICE_SECURE_OPEN, FILE_DEVICE_UNKNOWN, NT_SUCCESS, NTSTATUS, PCUNICODE_STRING,
    PDRIVER_OBJECT, STATUS_SUCCESS,
};

use crate::display::Displayable;
use crate::unicode::UnicodeString;

const DOS_NAME: &CStr = c"\\??\\LogDrvDev";
const DEVICE_NAME: &CStr = c"\\Device\\LogDrvDev";

fn delete_device(driver: PDRIVER_OBJECT) {
    let mut device_name = UnicodeString::from(DEVICE_NAME);

    let status = unsafe { IoDeleteSymbolicLink(&mut device_name.native) };
    if !NT_SUCCESS(status) {
        log!("Failed to remove symlink: {status}");
    }

    if let Some(driver) = unsafe { driver.as_ref() } {
        let device = driver.DeviceObject;
        if !device.is_null() {
            unsafe {
                IoDeleteDevice(device);
            }
        }
    }
}

/// # Safety
/// This is the entry point for the driver. It must be called by the OS.
#[unsafe(export_name = "DriverEntry")]
pub unsafe extern "system" fn driver_entry(
    driver: PDRIVER_OBJECT,
    registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    let driver_ref = match unsafe { driver.as_mut() } {
        Some(d) => d,
        None => {
            log!("driver_entry: PDRIVER_OBJECT is null");
            return STATUS_SUCCESS;
        }
    };

    driver_ref.DriverUnload = Some(driver_unload);

    let registry_path = match unsafe { registry_path.as_ref() } {
        Some(r) => r.display(),
        None => {
            log!("driver_entry: registry_path is null");
            "".to_string()
        }
    };

    log!(
        "driver_entry {:?}, registry_path={registry_path:?}",
        driver_ref.DriverName.display(),
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
        delete_device(driver);
        return status;
    }

    STATUS_SUCCESS
}

/// # Safety
/// This is the unload function for the driver. It must be called by the OS.
pub unsafe extern "C" fn driver_unload(driver: PDRIVER_OBJECT) {
    let driver_ref = match unsafe { driver.as_ref() } {
        Some(d) => d,
        None => {
            log!("driver_unload: PDRIVER_OBJECT is null");
            return;
        }
    };

    log!("driver_unload {:?}", driver_ref.DriverName.display());

    delete_device(driver);
}
