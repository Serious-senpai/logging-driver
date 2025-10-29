mod ioctl;
mod read;

use wdk_sys::{DEVICE_OBJECT, IO_STACK_LOCATION, IRP, STATUS_INVALID_DEVICE_REQUEST};

use crate::error::RuntimeError;
use crate::handlers::irp::ioctl::DeviceControlHandler;
use crate::handlers::irp::read::ReadHandler;
use crate::state::DeviceExtension;

pub trait IrpHandler<'a> {
    const CODE: u32;

    fn new(
        device: &'a DEVICE_OBJECT,
        extension: &'a DeviceExtension,
        irp: &'a mut IRP,
        irpsp: &'a mut IO_STACK_LOCATION,
    ) -> Result<Self, RuntimeError>
    where
        Self: Sized;

    /// # Safety
    /// This handler will eventually be called by the OS when handling an IRP.
    fn handle(&mut self) -> Result<(), RuntimeError>;
}

macro_rules! _irp_handle {
    ($device:expr, $extension:expr, $irp:expr, $irpsp:expr, $($Handler:tt,)*) => {
        match $irpsp.MajorFunction.into() {
            $(
                $Handler::CODE => {
                    let mut handler = $Handler::new(
                        $device,
                        $extension,
                        $irp,
                        $irpsp,
                    )?;
                    handler.handle()
                },
            )*
            _ => Err(RuntimeError::Failure(STATUS_INVALID_DEVICE_REQUEST)),
        }
    };
}

pub fn irp_handler(
    device: &DEVICE_OBJECT,
    extension: &DeviceExtension,
    irp: &mut IRP,
    irpsp: &mut IO_STACK_LOCATION,
) -> Result<(), RuntimeError> {
    _irp_handle!(
        device,
        extension,
        irp,
        irpsp,
        ReadHandler,
        DeviceControlHandler,
    )
}
