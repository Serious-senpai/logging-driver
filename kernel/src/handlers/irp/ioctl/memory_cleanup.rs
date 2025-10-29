use common::ioctl::IOCTL_MEMORY_CLEANUP;
use wdk_sys::{DEVICE_OBJECT, IO_STACK_LOCATION, IRP};

use crate::error::RuntimeError;
use crate::handlers::DeviceExtension;
use crate::handlers::irp::ioctl::IoctlHandler;

pub struct MemoryCleanupHandler<'a> {
    _device: &'a DEVICE_OBJECT,
    _extension: &'a DeviceExtension,
    _irp: &'a mut IRP,
    _irpsp: &'a mut IO_STACK_LOCATION,
}

impl<'a> IoctlHandler<'a> for MemoryCleanupHandler<'a> {
    const CODE: u32 = IOCTL_MEMORY_CLEANUP;

    fn new(
        device: &'a DEVICE_OBJECT,
        extension: &'a DeviceExtension,
        irp: &'a mut IRP,
        irpsp: &'a mut IO_STACK_LOCATION,
    ) -> Result<Self, RuntimeError> {
        Ok(Self {
            _device: device,
            _extension: extension,
            _irp: irp,
            _irpsp: irpsp,
        })
    }

    fn handle(&mut self) -> Result<(), RuntimeError> {
        let old = {
            let mut inner = self._extension.inner.acquire();
            inner.memmap.take()
        };

        drop(old);
        Ok(())
    }
}
