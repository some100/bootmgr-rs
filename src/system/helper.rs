use alloc::{borrow::ToOwned, boxed::Box, format, string::String, vec::Vec};
use uefi::{
    CStr16, CString16, Handle, boot,
    data_types::EqStrUntilNul,
    fs::FileSystem,
    guid,
    proto::{
        device_path::{DevicePath, PoolDevicePath, build},
        media::{
            file::FileInfo,
            partition::{GptPartitionType, PartitionInfo},
        },
    },
    runtime,
};

use crate::error::BootError;

const BOOTMGR_GUID: uefi::Guid = guid!("23600d08-561e-4e68-a024-1d7d6e04ee4e");
const XBOOTLDR_PARTITION: uefi::Guid = guid!("bc13c2ff-59e6-4262-a352-b275fd6f7172");

pub fn is_target_partition(handle: &Handle) -> bool {
    // for filesystems that support partitioninfo, filter partitions by guid
    if let Ok(info) = boot::open_protocol_exclusive::<PartitionInfo>(*handle) {
        let Some(entry) = info.gpt_partition_entry() else {
            return false;
        };
        let guid = entry.partition_type_guid;
        if guid != GptPartitionType::EFI_SYSTEM_PARTITION
            && guid != GptPartitionType(XBOOTLDR_PARTITION)
        {
            return false;
        }
    };
    true
}

// returns an iterator that filters non matching or invalid files
pub fn read_filtered_dir(
    fs: &mut FileSystem,
    path: &CStr16,
    ext: &'static str,
) -> impl Iterator<Item = Box<FileInfo>> + use<> {
    fs.read_dir(path)
        .into_iter()
        .flatten()
        .filter_map(|x| x.ok())
        .filter(|x| !x.file_name().eq_str_until_nul(".") && !x.file_name().eq_str_until_nul(".."))
        .filter(move |x| {
            String::from(x.file_name())
                .to_ascii_lowercase()
                .ends_with(ext)
        })
        .filter(|x| x.file_size() > 0)
}

pub fn get_path_cstr(prefix: &CStr16, filename: &CStr16) -> CString16 {
    let path = format!("{prefix}\\{filename}");
    let path_cstr =
        CString16::try_from(&*path).expect("Invalid character or nul character found in input");
    path_cstr
}

pub fn get_arch() -> Option<String> {
    if cfg!(target_arch = "x86") {
        Some("x86".to_owned())
    } else if cfg!(target_arch = "x86_64") {
        Some("x64".to_owned())
    } else if cfg!(target_arch = "arm") {
        Some("arm".to_owned())
    } else if cfg!(target_arch = "aarch64") {
        Some("aa64".to_owned())
    } else {
        None // rust doesnt support itanium anyways
    }
}

pub fn get_device_path(
    dev_path: &DevicePath,
    path: CString16,
    vec: &mut Vec<u8>,
) -> Result<PoolDevicePath, BootError> {
    let path: &DevicePath = build::DevicePathBuilder::with_vec(vec)
        .push(&build::media::FilePath { path_name: &*path })?
        .finalize()?;
    Ok(dev_path.append_path(path)?)
}

pub fn set_variable_num(name: &CStr16, num: usize) -> uefi::Result<()> {
    let bytes = num.to_ne_bytes();
    runtime::set_variable(
        name,
        &runtime::VariableVendor(BOOTMGR_GUID),
        runtime::VariableAttributes::NON_VOLATILE | runtime::VariableAttributes::BOOTSERVICE_ACCESS,
        &bytes,
    )
}

pub fn get_variable_num(name: &CStr16) -> uefi::Result<usize> {
    let mut array = [0; size_of::<usize>()];
    let mut buf = [0; size_of::<usize>()];
    let var = match runtime::get_variable(name, &runtime::VariableVendor(BOOTMGR_GUID), &mut buf) {
        Ok((var, _)) => var,
        Err(e) => return Err(e.to_err_without_payload()),
    };
    array.copy_from_slice(var);
    Ok(usize::from_ne_bytes(array))
}
