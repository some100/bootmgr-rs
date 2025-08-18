// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Boot Loader Interface support module.
//!
//! The Boot Loader Interface is a specification with the purpose of allowing systemd and the boot loader
//! to interact with one another. This is achieved through setting UEFI variables, which provide a two way
//! communication channel for the boot loader and systemd. This allows boot loaders, such as systemd-boot,
//! to use a tool such as `bootctl` to set the timeout, or set the next boot option.
//!
//! Currently, `bootmgr-rs` only supports the former. This is indicated to `bootctl` through the feature flags.

use alloc::{format, string::ToString, vec::Vec};

use bitflags::bitflags;
use uefi::{boot, cstr16, guid, runtime::VariableVendor};

use crate::{
    BootResult,
    config::Config,
    system::{
        fs::get_partition_guid,
        helper::str_to_cstr,
        time::timer_usec,
        variable::{set_variable, set_variable_str, set_variable_u16_slice},
    },
};

/// The variable namespace for Boot Loader Interface UEFI variables.
const BLI_VENDOR: VariableVendor = VariableVendor(guid!("4a67b082-0a4c-41cf-b6c7-440b29bb8c4f"));

bitflags! {
    /// Feature flags for Boot Loader Interface.
    struct LoaderFeatures: u64 {
        const TIMEOUT = 1 << 0;
        const TIMEOUT_ONESHOT = 1 << 1;
        const ENTRY_DEFAULT = 1 << 2;
        const ENTRY_ONESHOT = 1 << 3;
        const BOOT_COUNTER = 1 << 4;
        const XBOOTLDR = 1 << 5;
        const RANDOM_SEED = 1 << 6;
        const MENU_DISABLED = 1 << 13;
    }
}

/// Export the variables at system initialization for Boot Loader Interface.
///
/// # Errors
///
/// May return an `Error` if the variable could not be set.
pub(crate) fn export_variables() -> BootResult<()> {
    let supported = LoaderFeatures::TIMEOUT
        | LoaderFeatures::TIMEOUT_ONESHOT
        | LoaderFeatures::BOOT_COUNTER
        | LoaderFeatures::XBOOTLDR;

    let time = str_to_cstr(&timer_usec().to_string())?;
    let partition_guid =
        get_partition_guid(boot::image_handle()).and_then(|x| str_to_cstr(&x.to_string()).ok());
    let info = str_to_cstr(&format!("bootmgr-rs {}", env!("CARGO_PKG_VERSION")))?;
    set_variable_str(
        cstr16!("LoaderTimeInitUSec"),
        Some(BLI_VENDOR),
        None,
        Some(&time),
    )?;
    set_variable(
        cstr16!("LoaderFeatures"),
        Some(BLI_VENDOR),
        None,
        Some(supported.bits()),
    )?;
    set_variable_str(
        cstr16!("LoaderDevicePartUUID"),
        Some(BLI_VENDOR),
        None,
        partition_guid.as_deref(),
    )?;
    set_variable_str(cstr16!("LoaderInfo"), Some(BLI_VENDOR), None, Some(&info))?;
    Ok(())
}

/// Immediately before executing the image, record the time after the loader finishes its work.
///
/// # Errors
///
/// May return an `Error` if the variable could not be set.
pub(crate) fn record_exit_time() -> BootResult<()> {
    let time = str_to_cstr(&timer_usec().to_string())?;
    set_variable_str(
        cstr16!("LoaderTimeExecUSec"),
        Some(BLI_VENDOR),
        None,
        Some(&time),
    )?;
    Ok(())
}

/// Set the loader entries based off the filenames.
///
/// # Errors
///
/// May return an `Error` if the variable could not be set.
pub(crate) fn set_loader_entries(configs: &[Config]) -> BootResult<()> {
    let filenames: Vec<_> = configs
        .iter()
        .flat_map(|x: &Config| str_to_cstr(&x.filename))
        .collect();
    let entries: Vec<_> = filenames
        .iter()
        .map(|x| x.to_u16_slice_with_nul())
        .flat_map(|x| x.iter().copied())
        .collect();
    set_variable_u16_slice(
        cstr16!("LoaderEntries"),
        Some(BLI_VENDOR),
        None,
        Some(&entries),
    )
}

/// Get the timeout variable from Boot Loader Interface, if there is any.
///
/// This function is disabled when testing on host to avoid causing a panic while unit tests for `BootConfig`
/// are being done.
///
/// May return `None` if the variable does not exist.
#[cfg(not(test))]
pub(crate) fn get_timeout_var() -> Option<i64> {
    use crate::system::variable::get_variable_str;

    let timeout = get_variable_str(cstr16!("LoaderConfigTimeout"), Some(BLI_VENDOR)).ok();
    let oneshot = get_variable_str(cstr16!("LoaderConfigTimeoutOneshot"), Some(BLI_VENDOR)).ok();

    oneshot.map_or_else(
        || timeout.and_then(|timeout| match_timeout(&timeout)),
        |oneshot| {
            let _ = set_variable_str(
                cstr16!("LoaderConfigTimeoutOneshot"),
                Some(BLI_VENDOR),
                None,
                None,
            );
            match_timeout(&oneshot)
        },
    )
}

/// Set the timeout variable from Boot Loader Interface.
///
/// This function is disabled when testing on host to avoid causing a panic while unit tests for `BootConfig`
/// are being done.
///
/// # Errors
///
/// May return an `Error` if the variable could not be set.
#[cfg(not(test))]
pub(crate) fn set_timeout_var(timeout: i64) -> BootResult<()> {
    let timeout = str_to_cstr(&timeout.to_string())?;
    set_variable_str(
        cstr16!("LoaderConfigTimeout"),
        Some(BLI_VENDOR),
        None,
        Some(&timeout),
    )
}

/// Match a BLI timeout string into a `bootmgr-rs` compatible timeout value.
#[cfg(not(test))]
fn match_timeout(timeout: &uefi::CStr16) -> Option<i64> {
    use uefi::data_types::EqStrUntilNul;

    if timeout.eq_str_until_nul("menu-force") {
        Some(-1)
    } else if timeout.eq_str_until_nul("menu-hidden") || timeout.eq_str_until_nul("menu-disabled") {
        Some(0)
    } else {
        timeout.to_string().parse().ok()
    }
}
