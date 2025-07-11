use alloc::{vec, vec::Vec};
use uefi::{
    CStr16, Status, guid,
    runtime::{self, VariableAttributes, VariableVendor},
};

const BOOTMGR_GUID: uefi::Guid = guid!("23600d08-561e-4e68-a024-1d7d6e04ee4e");

/// A value that can be stored in a UEFI variable.
pub trait UefiVariable: Sized {
    fn to_le_bytes(self) -> Vec<u8>;
    fn from_le_bytes(bytes: &[u8]) -> Self;
    fn default() -> Self;
}

impl UefiVariable for usize {
    fn to_le_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
    fn from_le_bytes(bytes: &[u8]) -> Self {
        let mut array = [0; size_of::<Self>()];
        array.copy_from_slice(bytes);
        Self::from_le_bytes(array)
    }
    fn default() -> Self {
        0
    }
}

impl UefiVariable for u64 {
    fn to_le_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
    fn from_le_bytes(bytes: &[u8]) -> Self {
        let mut array = [0; size_of::<Self>()];
        array.copy_from_slice(bytes);
        Self::from_le_bytes(array)
    }
    fn default() -> Self {
        0
    }
}

impl UefiVariable for u8 {
    fn to_le_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
    fn from_le_bytes(bytes: &[u8]) -> Self {
        let mut array = [0; size_of::<Self>()];
        array.copy_from_slice(bytes);
        Self::from_le_bytes(array)
    }
    fn default() -> Self {
        0
    }
}

/// Sets a UEFI variable to a [`UefiVariable`] given the name.
///
/// If None is specified for the vendor, then the variable will be searched for in a custom GUID space,
/// not the global variables vendor space. In other words, unless you are storing your own variables,
/// it may not be what you expect.
///
/// # Errors
///
/// May return an `Error` for many reasons, see [`runtime::set_variable`]
pub fn set_variable<T: UefiVariable>(
    name: &CStr16,
    vendor: Option<VariableVendor>,
    attrs: Option<VariableAttributes>,
    num: T
) -> uefi::Result<()> {
    let bytes = num.to_le_bytes();
    let vendor = match vendor {
        Some(vendor) => vendor,
        None => runtime::VariableVendor(BOOTMGR_GUID),
    };
    let attrs = match attrs {
        Some(attrs) => attrs,
        None => VariableAttributes::NON_VOLATILE | VariableAttributes::BOOTSERVICE_ACCESS,
    };
    runtime::set_variable(name, &vendor, attrs, &bytes)
}

/// Gets a UEFI variable of a [`UefiVariable`] given the name
///
/// If None is specified for the vendor, then the variable will be searched for in a custom GUID space,
/// not the global variables vendor space. In other words, unless you are storing your own variables,
/// it may not be what you expect.
///
/// # Errors
///
/// May return an `Error` for many reasons, see [`runtime::get_variable`]
pub fn get_variable<T: UefiVariable>(
    name: &CStr16,
    vendor: Option<VariableVendor>,
) -> uefi::Result<T> {
    let mut buf = vec![0; size_of::<T>()];
    let vendor = match vendor {
        Some(vendor) => vendor,
        None => runtime::VariableVendor(BOOTMGR_GUID),
    };
    match runtime::get_variable(name, &vendor, &mut buf) {
        Ok((var, _)) => Ok(T::from_le_bytes(var)),
        Err(e) if e.status() == Status::NOT_FOUND => Ok(T::default()), // pretend that we got all zeroes if its not found
        Err(e) => Err(e.to_err_without_payload()),
    }
}
