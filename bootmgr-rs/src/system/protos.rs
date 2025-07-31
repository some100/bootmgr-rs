//! UEFI protocols that are not implemented in the [`uefi`] crate.
//!
//! This exposes the following protocols:
//! - [`DevicetreeFixup`]
//! - [`SecurityArch`]
//! - [`Security2Arch`]
//!
//! Technically, it also provides [`ShimImageLoader`], however that isn't really used for anything as if Shim
//! is loaded, it will have already hooked onto `LoadImage` and such. It only exists to detect its existence.

use core::ffi::c_void;

use uefi::{
    Status, guid,
    proto::{
        device_path::{DevicePath, FfiDevicePath},
        unsafe_protocol,
    },
};

/// A "boolean" that is actually a [`u8`]. Used for FFI interop.
type Bool = u8;

/// A raw binding for `EFI_DT_FIXUP_PROTOCOL`. Provides only one function, which is to fixup DTB blobs.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct DevicetreeFixupProtocol {
    /// The version of the protocol.
    revision: u64,

    /// Applies firmware fixups to a buffer.
    fixup: unsafe extern "efiapi" fn(
        this: *mut Self,
        fdt: *mut c_void,
        buffer_size: *mut usize,
        flags: u32,
    ) -> Status,
}

impl DevicetreeFixupProtocol {
    /// The GUID of the protocol.
    const GUID: uefi::Guid = guid!("e617d64c-fe08-46da-f4dc-bbd5870c7300");
}

/// Devicetree fixup protocol.
///
/// In ARM hardware, devicetrees are used to supply information about the hardware to the software.
/// However, some of the properties of the hardware can only be known at boot time. Therefore, the firmware
/// may apply fixups to the devicetree in order for it to be more accurate and aligned with the hardware.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[unsafe_protocol(DevicetreeFixupProtocol::GUID)]
pub struct DevicetreeFixup(DevicetreeFixupProtocol);

impl DevicetreeFixup {
    /// Apply fixups to a devicetree buffer.
    ///
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

/// The raw Security Arch protocol implementation.
///
/// You should rarely ever need to use this, unless you are installing a custom validator.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SecurityArchProtocol {
    /// Check the authentication status of a file using the `auth_status` parameter.
    ///
    /// Very rarely should you ever need to use this directly, unless you are hijacking it and replacing it with a
    /// custom validator.
    pub auth_state: unsafe extern "efiapi" fn(
        this: *const Self,
        auth_status: u32,
        file: *const FfiDevicePath,
    ) -> Status,
}

impl SecurityArchProtocol {
    /// The GUID of the protocol.
    const GUID: uefi::Guid = guid!("a46423e3-4617-49f1-b9ff-d1bfa9115839");
}

/// Security Arch Protocol.
///
/// When Secure Boot is enabled, the Security Arch protocols are responsible for ensuring that files are authenticated
/// according to platform security policy.
///
/// Its main purpose is to authenticate files according to abstracted platform specific security policies.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[unsafe_protocol(SecurityArchProtocol::GUID)]
pub struct SecurityArch(SecurityArchProtocol);

impl SecurityArch {
    /// Check the authentication status of a file using the `auth_status` parameter.
    ///
    /// You should never need to use this, `LoadImage` will call it automatically whenever UEFI Secure Boot is enabled.
    pub fn auth_state(&self, auth_status: u32, file: &DevicePath) -> Status {
        let file = file.as_ffi_ptr();
        unsafe { (self.0.auth_state)(&raw const self.0, auth_status, file) }
    }

    /// Get a clone of the inner raw [`SecurityArchProtocol`].
    #[must_use = "Has no effect if the result is unused"]
    pub fn get_inner(&self) -> &SecurityArchProtocol {
        &self.0
    }

    /// Get a mutable reference to the inner raw [`SecurityArchProtocol`].
    pub const fn get_inner_mut(&mut self) -> &mut SecurityArchProtocol {
        &mut self.0
    }
}

/// The raw Security2 Arch protocol implementation.
///
/// You should rarely ever need to use this, unless you are installing a custom validator.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Security2ArchProtocol {
    /// Check the authentication status of a file from either a raw pointer to an [`FfiDevicePath`], or
    /// a file buffer.
    ///
    /// Very rarely should you ever need to use this directly, unless you are hijacking it and replacing it with a
    /// custom validator.
    pub authentication: unsafe extern "efiapi" fn(
        this: *const Self,
        device_path: *const FfiDevicePath,
        file_buffer: *mut c_void,
        file_size: usize,
        boot_policy: Bool,
    ) -> Status,
}

impl Security2ArchProtocol {
    /// The GUID of the protocol.
    const GUID: uefi::Guid = guid!("94ab2f58-1438-4ef1-9152-18941a3a0e68");
}

/// Security2 Arch Protocol.
///
/// When Secure Boot is enabled, the Security Arch protocols are responsible for ensuring that files are authenticated
/// according to platform security policy.
///
/// Its main purpose is to authenticate files according to the security policy of the firmware.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
#[unsafe_protocol(Security2ArchProtocol::GUID)]
pub struct Security2Arch(Security2ArchProtocol);

impl Security2Arch {
    /// Check the authentication status of a file from either a reference to a [`DevicePath`], or a mutable slice
    /// of a file buffer.
    ///
    /// You should never need to use this, `LoadImage` will call it automatically whenever UEFI Secure Boot is enabled.
    pub fn authentication(
        &self,
        device_path: Option<&DevicePath>,
        file_buffer: &mut [u8],
        boot_policy: bool,
    ) -> Status {
        let device_path = device_path.map_or(core::ptr::null(), DevicePath::as_ffi_ptr);
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

    /// Get a shared reference to the inner raw [`Security2ArchProtocol`].
    #[must_use = "Has no effect if the result is unused"]
    pub fn get_inner(&self) -> &Security2ArchProtocol {
        &self.0
    }

    /// Get a mutable reference to the inner raw [`Security2ArchProtocol`].
    pub const fn get_inner_mut(&mut self) -> &mut Security2ArchProtocol {
        &mut self.0
    }
}

/// The raw Shim Image Loader protocol.
///
/// None of this is actually used, since Shim loader hooks onto `LoadImage` directly.
/// This is here so we can detect its existence for Shim v16+
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ShimImageLoaderProtocol {
    /// Load an image. The parameters are identical to the `uefi-raw` `LoadImage` implementation.
    pub load_image: unsafe extern "efiapi" fn(
        boot_policy: Bool,
        parent: *mut c_void,
        device_path: *mut FfiDevicePath,
        src: *mut c_void,
        src_size: usize,
        image: *mut c_void,
    ),
    /// Start an image. The parameters are identical to the `uefi-raw` `StartImage` implementation.
    pub start_image: unsafe extern "efiapi" fn(
        image: *mut c_void,
        exit_data_size: *mut usize,
        exit_data: *mut u16,
    ),
    /// Exit the image. The parameters are identical to the `uefi-raw` `Exit` implementation.
    pub exit: unsafe extern "efiapi" fn(
        image: *mut c_void,
        status: Status,
        exit_data_size: usize,
        exit_data: *mut u16,
    ),

    /// Unload an image. The parameters are identical to the `uefi-raw` `UnloadImage` implementation.
    pub unload_image: unsafe extern "efiapi" fn(image: *mut c_void),
}

impl ShimImageLoaderProtocol {
    /// The GUID of the protocol.
    const GUID: uefi::Guid = guid!("1f492041-fadb-4e59-9e57-7cafe73a55ab");
}

/// Shim Image Loader protocol.
///
/// This is never used directly, since Shim will automatically hook onto `LoadImage` and other similar functions.
#[derive(Clone, Debug)]
#[repr(transparent)]
#[unsafe_protocol(ShimImageLoaderProtocol::GUID)]
pub struct ShimImageLoader(ShimImageLoaderProtocol);
