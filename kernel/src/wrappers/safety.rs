use wdk_sys::ntddk::{IoCreateSymbolicLink, IoDeleteSymbolicLink, PsSetCreateProcessNotifyRoutine};
use wdk_sys::{BOOLEAN, HANDLE, NT_SUCCESS, PASSIVE_LEVEL};

use crate::error::RuntimeError;
use crate::wrappers::irql::irql_requires;
use crate::wrappers::strings::UnicodeString;

pub fn delete_symbolic_link(name: &UnicodeString) -> Result<(), RuntimeError> {
    irql_requires(PASSIVE_LEVEL)?;

    let status = unsafe {
        let mut value = name.native().into_inner();

        // This implementation relies on the fact that `IoDeleteSymbolicLink` does not modify the string
        IoDeleteSymbolicLink(&mut value)
    };
    if !NT_SUCCESS(status) {
        return Err(RuntimeError::Failure(status));
    }

    Ok(())
}

pub fn create_symbolic_link(
    symbolic_link_name: &UnicodeString,
    device_name: &UnicodeString,
) -> Result<(), RuntimeError> {
    irql_requires(PASSIVE_LEVEL)?;

    let status = unsafe {
        let mut sym_name = symbolic_link_name.native().into_inner();
        let mut dev_name = device_name.native().into_inner();

        // This implementation relies on the fact that `IoCreateSymbolicLink` does not modify the string
        IoCreateSymbolicLink(&mut sym_name, &mut dev_name)
    };
    if !NT_SUCCESS(status) {
        return Err(RuntimeError::Failure(status));
    }

    Ok(())
}

fn _set_create_process_notify<const REMOVE: u8>(
    handler: unsafe extern "C" fn(HANDLE, HANDLE, BOOLEAN),
) -> Result<(), RuntimeError> {
    irql_requires(PASSIVE_LEVEL)?;

    let status = unsafe { PsSetCreateProcessNotifyRoutine(Some(handler), REMOVE) };
    if !NT_SUCCESS(status) {
        return Err(RuntimeError::Failure(status));
    }

    Ok(())
}

pub fn add_create_process_notify(
    handler: unsafe extern "C" fn(HANDLE, HANDLE, BOOLEAN),
) -> Result<(), RuntimeError> {
    _set_create_process_notify::<0>(handler)
}

pub fn remove_create_process_notify(
    handler: unsafe extern "C" fn(HANDLE, HANDLE, BOOLEAN),
) -> Result<(), RuntimeError> {
    _set_create_process_notify::<1>(handler)
}
