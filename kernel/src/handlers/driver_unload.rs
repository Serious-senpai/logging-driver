use core::ptr::null_mut;
use core::sync::atomic::Ordering;

use wdk_sys::{BOOLEAN, DRIVER_OBJECT, HANDLE};

use crate::config::DRIVER;
use crate::displayer::ForeignDisplayer;
use crate::error::RuntimeError;
use crate::handlers::delete_device;
use crate::log;
use crate::wrappers::safety::remove_create_process_notify;

pub fn driver_unload(
    driver: &mut DRIVER_OBJECT,
    process_notify: unsafe extern "C" fn(HANDLE, HANDLE, BOOLEAN),
) -> Result<(), RuntimeError> {
    log!(
        "driver_unload {:?}",
        ForeignDisplayer::Unicode(&driver.DriverName),
    );
    delete_device(driver);
    DRIVER.store(null_mut(), Ordering::SeqCst);

    remove_create_process_notify(process_notify).inspect_err(|e| {
        log!("Failed to remove process notify: {e}");
    })?;

    Ok(())
}
