use core::ffi::c_void;
use core::ptr;

use common::ioctl::{IOCTL_MEMORY_INITIALIZE, MemoryInitialize, SharedMemory};
use wdk_sys::_MODE::UserMode;
use wdk_sys::ntddk::{MmMapViewInSystemSpace, ObReferenceObjectByHandle, ObfDereferenceObject};
use wdk_sys::{
    DEVICE_OBJECT, EVENT_MODIFY_STATE, ExEventObjectType, IO_STACK_LOCATION, IRP, NT_SUCCESS,
    SECTION_MAP_READ, SECTION_MAP_WRITE, STATUS_INVALID_PARAMETER, SYNCHRONIZE,
};

use crate::error::RuntimeError;
use crate::handlers::DeviceExtension;
use crate::handlers::irp::ioctl::IoctlHandler;
use crate::log;
use crate::state::MemoryMap;

pub struct MemoryInitializeHandler<'a> {
    _device: &'a DEVICE_OBJECT,
    _extension: &'a DeviceExtension,
    _irp: &'a mut IRP,
    _irpsp: &'a mut IO_STACK_LOCATION,
    _input_length: usize,

    _section: *mut c_void,
    _event: *mut c_void,
}

impl<'a> IoctlHandler<'a> for MemoryInitializeHandler<'a> {
    const CODE: u32 = IOCTL_MEMORY_INITIALIZE;

    fn new(
        device: &'a DEVICE_OBJECT,
        extension: &'a DeviceExtension,
        irp: &'a mut IRP,
        irpsp: &'a mut IO_STACK_LOCATION,
    ) -> Result<Self, RuntimeError> {
        let input_length = unsafe { irpsp.Parameters.DeviceIoControl.InputBufferLength };
        Ok(Self {
            _device: device,
            _extension: extension,
            _irp: irp,
            _irpsp: irpsp,
            _input_length: input_length.try_into()?,
            _section: ptr::null_mut(),
            _event: ptr::null_mut(),
        })
    }

    fn handle(&mut self) -> Result<(), RuntimeError> {
        if self._input_length != size_of::<MemoryInitialize>() {
            return Err(RuntimeError::Failure(STATUS_INVALID_PARAMETER));
        }

        let input = match unsafe {
            let ptr = self._irp.AssociatedIrp.SystemBuffer as *const MemoryInitialize;
            ptr.as_ref()
        } {
            Some(input) => input,
            None => return Err(RuntimeError::Failure(STATUS_INVALID_PARAMETER)),
        };

        if usize::try_from(input.view_size)? < size_of::<SharedMemory>() {
            return Err(RuntimeError::Failure(STATUS_INVALID_PARAMETER));
        }

        log!("Received memory with view size {}", input.view_size);

        // Reference the section and event handles from user mode.
        let status = unsafe {
            ObReferenceObjectByHandle(
                input.section,
                SECTION_MAP_READ | SECTION_MAP_WRITE,
                ptr::null_mut(),
                UserMode.try_into()?,
                &mut self._section,
                ptr::null_mut(),
            )
        };
        if !NT_SUCCESS(status) {
            return Err(RuntimeError::Failure(status));
        }

        let status = unsafe {
            ObReferenceObjectByHandle(
                input.event,
                EVENT_MODIFY_STATE | SYNCHRONIZE,
                *ExEventObjectType,
                UserMode.try_into()?,
                &mut self._event,
                ptr::null_mut(),
            )
        };
        if !NT_SUCCESS(status) {
            return Err(RuntimeError::Failure(status));
        }

        // Map the section into system space.
        let mut mapped_base = ptr::null_mut();
        let mut view_size = input.view_size;
        let status =
            unsafe { MmMapViewInSystemSpace(self._section, &mut mapped_base, &mut view_size) };
        if !NT_SUCCESS(status) {
            return Err(RuntimeError::Failure(status));
        }

        // Build the new mapping object.
        let new_map = unsafe {
            MemoryMap::initialize(
                self._section,
                self._event,
                mapped_base as *mut SharedMemory,
                view_size,
            )
        };

        // Swap it into place under the spinlock, but drop the old mapping AFTER releasing the lock.
        let old = {
            let mut inner = self._extension.inner.acquire();
            inner.memmap.replace(new_map)
        };

        // Now that the lock is released (IRQL restored), drop the old mapping safely at PASSIVE_LEVEL.
        drop(old);

        Ok(())
    }

    fn on_failure(&mut self) {
        unsafe {
            if !self._event.is_null() {
                ObfDereferenceObject(self._event);
                self._event = ptr::null_mut();
            }
            if !self._section.is_null() {
                ObfDereferenceObject(self._section);
                self._section = ptr::null_mut();
            }
        }
    }
}
