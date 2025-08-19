// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Boot Loader Interface support module.
//!
//! The Boot Loader Interface is a specification with the purpose of allowing systemd and the boot loader
//! to interact with one another. This is achieved through setting UEFI variables, which provide a two way
//! communication channel for the boot loader and systemd. This allows boot loaders, such as systemd-boot,
//! to use a tool such as `bootctl` to set the timeout, or set the next boot option.
//!
//! This module provides an essentially complete implementation of this interface as per the
//! [Boot Loader Interface](https://systemd.io/BOOT_LOADER_INTERFACE/) specification. Only the features necessary
//! for interaction with a tool like `bootctl` are implemented in this module. Even if `bootctl` defines more
//! features, these are purely for reporting in `bootctl status`.

use alloc::{format, string::ToString, vec::Vec};

use bitflags::bitflags;
use sha2::{Digest, Sha256};
use uefi::{
    CStr16, boot, cstr16,
    data_types::EqStrUntilNul,
    guid,
    proto::rng::Rng,
    runtime::{self, VariableAttributes, VariableVendor},
};

use crate::{
    BootResult,
    config::Config,
    system::{
        fs::{UefiFileSystem, get_partition_guid},
        helper::{locate_protocol, str_to_cstr},
        time::timer_usec,
        variable::{get_variable_str, set_variable, set_variable_str, set_variable_u16_slice},
    },
};

/// The variable namespace for Boot Loader Interface UEFI variables.
const BLI_VENDOR: VariableVendor = VariableVendor(guid!("4a67b082-0a4c-41cf-b6c7-440b29bb8c4f"));

/// The attributes for a variable accessible at boot and runtime, but is not persistent.
///
/// Some of these variables do not need to persist because they only must be accessible to `bootctl` at runtime. For example,
/// `LoaderTimeInitUSec` does not need to persist between boots, same as `LoaderFeatures`, which is already set everytime
/// at initialization anyways. This avoids the overhead of storing these variables in NVRAM, which could have an impact
/// on boot times.
const VOLATILE_ATTRS: VariableAttributes =
    VariableAttributes::BOOTSERVICE_ACCESS.union(VariableAttributes::RUNTIME_ACCESS);

/// The path of the random seed.
const RANDOM_SEED_PATH: &CStr16 = cstr16!("\\loader\\random-seed");

bitflags! {
    /// Feature flags for Boot Loader Interface.
    ///
    /// Note that half of these features aren't necessary for interacting with `bootctl`, they only exist to
    /// specify the "features" that the bootloader supports in `bootctl status`.
    ///
    /// Some of these features are flagged as not supported, even if `bootmgr-rs` may support these features,
    /// due to different paths that `bootmgr-rs` may use or because it uses a different configuration format
    /// (such as `LOAD_DRIVER` and `SAVED_ENTRY`).
    ///
    /// The only feature flags that are necessary for direct interaction with `bootctl` are `TIMEOUT`,
    /// `TIMEOUT_ONESHOT`, `ENTRY_DEFAULT`, `ENTRY_ONESHOT`, `BOOT_COUNTER`, `XBOOTLDR`, `RANDOM_SEED`,
    /// and `MENU_DISABLED`.
    struct LoaderFeatures: u64 {
        const TIMEOUT = 1 << 0;
        const TIMEOUT_ONESHOT = 1 << 1;
        const ENTRY_DEFAULT = 1 << 2;
        const ENTRY_ONESHOT = 1 << 3;
        const BOOT_COUNTER = 1 << 4;
        const XBOOTLDR = 1 << 5;
        const RANDOM_SEED = 1 << 6;
        const LOAD_DRIVER = 1 << 7;
        const SORT_KEY = 1 << 8;
        const SAVED_ENTRY = 1 << 9;
        const DEVICETREE = 1 << 10;
        const SECUREBOOT_ENROLL = 1 << 11;
        const RETAIN_SHIM = 1 << 12;
        const MENU_DISABLED = 1 << 13;
        const MULTI_PROFILE_UKI = 1 << 14;
        const REPORT_URL = 1 << 15;
        const TYPE1_UKI = 1 << 16;
        const TYPE1_UKI_URL = 1 << 17;
        const TPM2_ACTIVE_PCR_BANKS = 1 << 18;
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
        | LoaderFeatures::ENTRY_DEFAULT
        | LoaderFeatures::ENTRY_ONESHOT
        | LoaderFeatures::BOOT_COUNTER
        | LoaderFeatures::XBOOTLDR
        | LoaderFeatures::RANDOM_SEED
        | LoaderFeatures::SORT_KEY
        | LoaderFeatures::DEVICETREE
        | LoaderFeatures::RETAIN_SHIM
        | LoaderFeatures::MENU_DISABLED; // this is frontend dependent, depending on how input events are handled.

    let time = str_to_cstr(&timer_usec().to_string())?;
    let partition_guid =
        get_partition_guid(boot::image_handle()).and_then(|x| str_to_cstr(&x.to_string()).ok());
    let info = str_to_cstr(&format!("bootmgr-rs {}", env!("CARGO_PKG_VERSION")))?;
    set_variable_str(
        cstr16!("LoaderTimeInitUSec"),
        Some(BLI_VENDOR),
        Some(VOLATILE_ATTRS),
        Some(&time),
    )?;
    set_variable(
        cstr16!("LoaderFeatures"),
        Some(BLI_VENDOR),
        Some(VOLATILE_ATTRS),
        Some(supported.bits()),
    )?;
    set_variable_str(
        cstr16!("LoaderDevicePartUUID"),
        Some(BLI_VENDOR),
        Some(VOLATILE_ATTRS),
        partition_guid.as_deref(),
    )?;
    set_variable_str(
        cstr16!("LoaderInfo"),
        Some(BLI_VENDOR),
        Some(VOLATILE_ATTRS),
        Some(&info),
    )?;
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
        Some(VOLATILE_ATTRS),
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
        Some(VOLATILE_ATTRS),
        Some(&entries),
    )
}

/// Get the default entry based off the BLI identifier.
///
/// May return `None` if the variable does not exist.
pub(crate) fn get_default_entry(configs: &[Config]) -> Option<usize> {
    let default = get_variable_str(cstr16!("LoaderEntryDefault"), Some(BLI_VENDOR)).ok();
    let oneshot = get_variable_str(cstr16!("LoaderEntryOneShot"), Some(BLI_VENDOR)).ok();

    oneshot.map_or_else(
        || {
            default.and_then(|default| {
                configs
                    .iter()
                    .position(|x| x.filename.eq_str_until_nul(&default))
            })
        },
        |oneshot| {
            configs
                .iter()
                .position(|x| x.filename.eq_str_until_nul(&oneshot))
        },
    )
}

/// Set the default entry from Boot Loader Interface.
///
/// This function is disabled when testing on host to avoid causing a panic while unit tests for `BootConfig`
/// are being done.
///
/// # Errors
///
/// May return an `Error` if the variable could not be set.
pub(crate) fn set_default_entry(configs: &[Config], idx: usize) -> BootResult<()> {
    let timeout = str_to_cstr(&configs[idx].filename)?;
    set_variable_str(
        cstr16!("LoaderEntryDefault"),
        Some(BLI_VENDOR),
        None,
        Some(&timeout),
    )
}

/// Get the timeout variable from Boot Loader Interface, if there is any.
///
/// This has `dead_code` allowed since in tests, this will produce a false warning since the UEFI-specific code using
/// this function is not included.
///
/// May return `None` if the variable does not exist.
#[allow(dead_code)]
pub(crate) fn get_timeout_var() -> Option<i64> {
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
/// This has `dead_code` allowed since in tests, this will produce a false warning since the UEFI-specific code using
/// this function is not included.
///
/// # Errors
///
/// May return an `Error` if the variable could not be set.
#[allow(dead_code)]
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
fn match_timeout(timeout: &uefi::CStr16) -> Option<i64> {
    if timeout.eq_str_until_nul("menu-force") {
        Some(-1)
    } else if timeout.eq_str_until_nul("menu-hidden") || timeout.eq_str_until_nul("menu-disabled") {
        Some(0)
    } else {
        timeout.to_string().parse().ok()
    }
}

/// Generate a random seed given the system RNG, the on-disk seed, and the system token.
///
/// This will generate a random seed as per the Boot Loader Interface, which list that the random seed should be hashed
/// with the available source of entropy (the Rng protocol), the random seed, and the system token. Additionally, it will
/// also hash in the available time (microseconds since boot). Entropy is gathered on a best-effort basis, which means
/// that errors that may occur from the available sources are ignored.
///
/// # Errors
///
/// May return an `Error` if the filesystem could not be opened, or the finalized seed could not be written back to.
pub(crate) fn generate_random_seed() -> BootResult<()> {
    let mut fs = UefiFileSystem::from_image_fs()?;

    let mut hasher = Sha256::new();

    if let Ok(content) = fs.read(RANDOM_SEED_PATH) {
        hasher.update(&content);
    }

    if let Ok((token, _)) = runtime::get_variable_boxed(cstr16!("LoaderSystemToken"), &BLI_VENDOR) {
        hasher.update(&token);
    }

    if let Ok(mut rng) = locate_protocol::<Rng>() {
        let mut buf = [0; 64];
        let _ = rng.get_rng(None, &mut buf);

        hasher.update(buf);
    }

    // Add in the current time in there just for fun (and extra entropy)
    hasher.update(timer_usec().to_le_bytes());

    let result = hasher.finalize();

    let _ = fs.delete(RANDOM_SEED_PATH);
    fs.create(RANDOM_SEED_PATH)?;
    fs.write(RANDOM_SEED_PATH, &result)?;

    Ok(())
}
