use alloc::string::String;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BootError {
    #[error("UEFI Error")]
    Uefi(#[from] uefi::Error),
    #[error("Filesystem Error")]
    Fs(#[from] uefi::fs::Error),
    #[error("From Str Error")]
    FromStr(#[from] uefi::data_types::FromStrError),
    #[error("Device Path Build Error")]
    Build(#[from] uefi::proto::device_path::build::BuildError),
    #[error("Device Path Utilities Error")]
    DevPathUtil(#[from] uefi::proto::device_path::DevicePathUtilitiesError),
    #[error("PE Section Parse Error: {0}")]
    Pe(pelite::Error),
    #[error("Hive Parse Error")]
    Hive(#[from] nt_hive::NtHiveError),
    #[error("Error: {0}")]
    Generic(&'static str),
    #[error("Error: {0}")]
    GenericOwned(String),
}
