use core::marker::PhantomData;

use wdk_sys::{DEVICE_OBJECT, IO_STACK_LOCATION, IRP, IRP_MJ_CLEANUP};

use crate::error::RuntimeError;
use crate::handlers::irp::IrpHandler;
use crate::state::DeviceExtension;

pub struct CleanupHandler<'a> {
    _phantom: PhantomData<&'a ()>,
}

impl<'a> IrpHandler<'a> for CleanupHandler<'a> {
    const CODE: u32 = IRP_MJ_CLEANUP;

    fn new(
        _: &'a DEVICE_OBJECT,
        _: &'a DeviceExtension,
        _: &'a mut IRP,
        _: &'a mut IO_STACK_LOCATION,
    ) -> Result<Self, RuntimeError> {
        Ok(Self {
            _phantom: PhantomData,
        })
    }

    fn handle(&mut self) -> Result<(), RuntimeError> {
        Ok(())
    }
}
