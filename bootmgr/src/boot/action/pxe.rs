// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Provides [`get_pxe_offer`] which obtains offers through DHCP and parses the response into a [`Config`]

use alloc::{format, string::ToString};
use core::{ffi::CStr, net::Ipv4Addr};

use uefi::proto::network::pxe::{BaseCode, BootstrapType, DhcpV4Packet};

use crate::{
    BootResult,
    boot::action::BootAction,
    config::{Config, builder::ConfigBuilder, parsers::Parsers},
    system::helper::locate_protocol,
};

/// Attempts to obtain a response through PXE DHCP. If one is obtained, create a [`Config`] for it.
///
/// PXE works through using DHCP to provide the boot file, possibly parameters, and the IP address where
/// the file is hosted. This function provides a basic means to obtain a boot file from a DHCP server, as
/// well as the server where the boot file was obtained from. Respectively, these are stored in the EFI
/// and filename fields of the [`Config`].
///
/// If the [`Config`] is an HTTP boot configuration, detected by checking if the boot name starts with
/// `http://` or `https://`, those will not return a [`Config`]. This is because there is no support for
/// HTTP boot, and if a DHCP server is replying with HTTP boot configurations, there is likely some type of
/// mismatch or error. If HTTP boot is needed, chainload a more feature complete loader like `iPXE`.
///
/// # Errors
///
/// May return an `Error` if the firmware does not support [`BaseCode`].
pub fn get_pxe_offer() -> BootResult<Option<Config>> {
    let mut base_code = locate_protocol::<BaseCode>()?;
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

        if !file.starts_with("http://") && !file.starts_with("https://") {
            let addr = Ipv4Addr::from(reply.bootp_si_addr).to_string();
            let title = format!("PXE Boot: {file}");

            let config = ConfigBuilder::new(addr, "")
                .efi_path(file)
                .title(title)
                .action(BootAction::BootTftp)
                .origin(Parsers::Special)
                .build();

            return Ok(Some(config));
        }
    }

    Ok(None)
}
