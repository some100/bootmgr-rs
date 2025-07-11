#![allow(dead_code)]
// Protocols that were not implemented in uefi-rs.
// Some protocols are currently unused for a future release.

use core::ffi::c_void;

use uefi::{
    Status, guid,
    proto::{
        device_path::{DevicePath, FfiDevicePath},
        unsafe_protocol,
    },
};

type BOOL = u8;

#[derive(Debug)]
#[repr(C)]
struct DevicetreeFixupProtocol {
    revision: u64,
    fixup: unsafe extern "efiapi" fn(
        this: *mut Self,
        fdt: *mut c_void,
        buffer_size: *mut usize,
        flags: u32,
    ) -> Status,
}

impl DevicetreeFixupProtocol {
    const GUID: uefi::Guid = guid!("e617d64c-fe08-46da-f4dc-bbd5870c7300");
}

#[derive(Debug)]
#[repr(transparent)]
#[unsafe_protocol(DevicetreeFixupProtocol::GUID)]
pub struct DevicetreeFixup(DevicetreeFixupProtocol);

impl DevicetreeFixup {
    pub fn fixup(&mut self, fdt: *mut c_void, buffer_size: &mut usize, flags: u32) -> Status {
        unsafe { (self.0.fixup)(&mut self.0, fdt, buffer_size, flags) }
    }
}

#[derive(Debug)]
#[repr(C)]
struct SecurityArchProtocol {
    auth_state: unsafe extern "efiapi" fn(
        this: *const Self,
        auth_status: u32,
        file: *const FfiDevicePath,
    ) -> Status,
}

impl SecurityArchProtocol {
    const GUID: uefi::Guid = guid!("a46423e3-4617-49f1-b9ff-d1bfa9115839");
}

#[derive(Debug)]
#[repr(transparent)]
#[unsafe_protocol(SecurityArchProtocol::GUID)]
pub struct SecurityArch(SecurityArchProtocol);

impl SecurityArch {
    pub fn auth_state(&self, auth_status: u32, file: &DevicePath) -> Status {
        let file = file.as_ffi_ptr();
        unsafe { (self.0.auth_state)(&self.0, auth_status, file) }
    }
}

#[derive(Debug)]
#[repr(C)]
struct Security2ArchProtocol {
    authentication: unsafe extern "efiapi" fn(
        this: *const Self,
        device_path: *const FfiDevicePath,
        file_buffer: *mut c_void,
        file_size: usize,
        boot_policy: BOOL,
    ) -> Status,
}

impl Security2ArchProtocol {
    const GUID: uefi::Guid = guid!("94ab2f58-1438-4ef1-9152-18941a3a0e68");
}

#[derive(Debug)]
#[repr(transparent)]
#[unsafe_protocol(Security2ArchProtocol::GUID)]
pub struct Security2Arch(Security2ArchProtocol);

impl Security2Arch {
    pub fn authentication(
        &self,
        device_path: &DevicePath,
        file_buffer: &mut [u8],
        boot_policy: bool,
    ) -> Status {
        let device_path = device_path.as_ffi_ptr();
        let file_size = file_buffer.len();
        let file_buffer = file_buffer.as_mut_ptr() as *mut c_void;
        unsafe {
            (self.0.authentication)(
                &self.0,
                device_path,
                file_buffer,
                file_size,
                BOOL::from(boot_policy),
            )
        }
    }
}
