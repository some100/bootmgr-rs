//! The boot loader for network executables (really EFI loaded over network)
//!
//! It downloads a file from a TFTP server, then uses that file buffer as an EFI executable.
//! The current support for PXE is insanely basic, and any more complex configurations (such as HTTP boot)
//! should use a more comprehensive PXE loader such as `iPXE` instead. This should be preferred even if your
//! configuration is very simple.

use alloc::vec;

use core::{net::Ipv4Addr, str::FromStr};

use uefi::{
    Handle, boot,
    proto::network::{IpAddress, pxe::BaseCode},
};

use crate::{
    BootResult,
    boot::{loader::LoadError, secure_boot::shim::shim_load_image},
    config::Config,
    system::helper::{bytes_to_cstr8, str_to_cstring},
};

/// Loads a boot option from a given [`Config`] through TFTP.
///
/// # Errors
///
/// May return an `Error` if the firmware does not support [`BaseCode`], or the
/// EFI executable is not a valid Latin-1 string, or the filename is not a valid
/// IP address, or [`boot::load_image`] fails.
pub fn load_boot_option(config: &Config) -> BootResult<Handle> {
    let handle = boot::get_handle_for_protocol::<BaseCode>()?;
    let mut base_code = boot::open_protocol_exclusive::<BaseCode>(handle)?;

    let addr_as_octets = Ipv4Addr::from_str(&config.filename)
        .map_err(LoadError::IpParse)?
        .octets();
    let addr = IpAddress::new_v4(addr_as_octets);

    if !base_code.mode().started() {
        // at this point it should already be started
        base_code.start(true)?;
    }

    let efi = config
        .efi
        .as_deref()
        .ok_or_else(|| LoadError::ConfigMissingEfi(config.filename.clone()))?;

    let filename = str_to_cstring(efi)?;
    let filename_bytes = filename.as_bytes_with_nul();
    let filename_cstr = bytes_to_cstr8(filename_bytes)?;

    // if its too big, its due to 32 bit platform limitations, and it would not be possible to allocate a buffer
    // greater than the pointer width max either way. truncating should generally be fine on 64 bit platforms though
    let size = match usize::try_from(base_code.tftp_get_file_size(&addr, filename_cstr)?) {
        Ok(size) => size,
        _ => usize::MAX,
    };

    let mut vec = vec![0; size];
    base_code.tftp_read_file(&addr, filename_cstr, Some(&mut vec))?;

    let src = boot::LoadImageSource::FromBuffer {
        buffer: &vec,
        file_path: None,
    };
    shim_load_image(boot::image_handle(), src)
}
