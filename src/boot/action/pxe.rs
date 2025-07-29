//! Provides [`get_pxe_offer`] which obtains offers through DHCP and parses the response into a [`Config`]

use core::{ffi::CStr, net::Ipv4Addr};

use alloc::{format, string::ToString};
use uefi::{
    boot,
    proto::network::pxe::{BaseCode, BootstrapType, DhcpV4Packet},
};

use crate::{
    BootResult,
    boot::action::BootAction,
    config::{Config, builder::ConfigBuilder},
};

/// Attempts to obtain a response through PXE DHCP. If one is obtained, create a [`Config`] for it.
///
/// # Errors
///
/// May return an `Error` if the firmware does not support [`BaseCode`].
pub fn get_pxe_offer() -> BootResult<Option<Config>> {
    let handle = boot::get_handle_for_protocol::<BaseCode>()?;
    let mut base_code = boot::open_protocol_exclusive::<BaseCode>(handle)?;
    if !base_code.mode().started() {
        base_code.start(false)?;
    }

    base_code.dhcp(true)?;

    let mut initial_layer = 0; // when starting a discover, use layer 0
    base_code.discover(BootstrapType::BOOTSTRAP, &mut initial_layer, false, None)?;

    if base_code.mode().pxe_reply_received() {
        let reply: &DhcpV4Packet = base_code.mode().pxe_reply().as_ref();
        let Ok(file) = CStr::from_bytes_with_nul(&reply.bootp_boot_file) else {
            return Ok(None);
        };
        let file = file.to_string_lossy();

        if !file.starts_with("http://") {
            let addr = Ipv4Addr::from(reply.bootp_si_addr).to_string();

            let config = ConfigBuilder::new(addr, "")
                .efi(file.clone())
                .title(format!("PXE Boot: {file}"))
                .action(BootAction::BootTftp)
                .build();

            return Ok(Some(config));
        }
    }

    Ok(None)
}
