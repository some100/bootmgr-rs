#![allow(dead_code, reason = "Unimplemented for a future release")]
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

type Bool = u8;

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
    /// # Safety
    ///
    /// You probably should not call this with a null pointer for fdt.
    pub unsafe fn fixup(
        &mut self,
        fdt: *mut c_void,
        buffer_size: &mut usize,
        flags: u32,
    ) -> Status {
        unsafe { (self.0.fixup)(&raw mut self.0, fdt, buffer_size, flags) }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct SecurityArchProtocol {
    pub auth_state: unsafe extern "efiapi" fn(
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
        unsafe { (self.0.auth_state)(&raw const self.0, auth_status, file) }
    }

    pub fn get_inner(&mut self) -> &mut SecurityArchProtocol {
        &mut self.0
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Security2ArchProtocol {
    pub authentication: unsafe extern "efiapi" fn(
        this: *const Self,
        device_path: *const FfiDevicePath,
        file_buffer: *mut c_void,
        file_size: usize,
        boot_policy: Bool,
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
        device_path: Option<&DevicePath>,
        file_buffer: &mut [u8],
        boot_policy: bool,
    ) -> Status {
        let device_path = if let Some(device_path) = device_path {
            device_path.as_ffi_ptr()
        } else {
            core::ptr::null() // NULL devicepath means unknown origin
        };
        let file_size = file_buffer.len();
        let file_buffer = file_buffer.as_mut_ptr().cast::<c_void>();
        unsafe {
            (self.0.authentication)(
                &raw const self.0,
                device_path,
                file_buffer,
                file_size,
                Bool::from(boot_policy),
            )
        }
    }

    pub fn get_inner(&mut self) -> &mut Security2ArchProtocol {
        &mut self.0
    }
}
