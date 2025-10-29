pub mod memory_cleanup;
pub mod memory_init;

use wdk_sys::{DEVICE_OBJECT, IO_STACK_LOCATION, IRP, NTSTATUS};

pub trait IoctlHandler {
    const CODE: u32;

    /// # Safety
    /// This handler will eventually be called by the OS when handling an IOCTL request
    /// (i.e. `IRP_MJ_DEVICE_CONTROL`).
    ///
    /// The implementation may perform unsafe operations as needed.
    unsafe fn handle(
        device: &mut DEVICE_OBJECT,
        irp: &mut IRP,
        irpsp: &IO_STACK_LOCATION,
        input_buffer_length: u32,
    ) -> NTSTATUS;
}
