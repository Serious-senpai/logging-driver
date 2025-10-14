#![no_std]

#[cfg(not(test))]
extern crate wdk_panic;

#[cfg(not(test))]
use wdk_alloc::WdkAllocator;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

use wdk::println;
use wdk_sys::{NTSTATUS, PCUNICODE_STRING, PDRIVER_OBJECT};

#[unsafe(export_name = "DriverEntry")]
pub unsafe extern "system" fn driver_entry(
    driver: PDRIVER_OBJECT,
    registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    let mut driver = unsafe { *driver };
    driver.DriverUnload = Some(driver_unload);

    let name = driver.DriverName;
    println!("Loaded driver {name:?} {registry_path:?}");
    0
}

pub unsafe extern "C" fn driver_unload(driver: PDRIVER_OBJECT) {
    let driver = unsafe { *driver };
    let name = driver.DriverName;
    println!("Unloaded driver {name:?}");
}
