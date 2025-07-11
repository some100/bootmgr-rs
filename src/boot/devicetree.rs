//! Handles devicetree installations and fixups.
//!
//! This is mostly based off of systemd-boot's implementation

use core::ffi::c_void;
use core::mem::ManuallyDrop;
use core::ptr::{NonNull, copy_nonoverlapping};

use uefi::boot::ScopedProtocol;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{CString16, guid, prelude::*};

use crate::system::fs::read;
use crate::system::helper::normalize_path;
use crate::system::protos::DevicetreeFixup;

const DTB_CONF_TABLE: uefi::Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");
const DTB_FIXUP_TABLE: uefi::Guid = guid!("e617d64c-fe08-46da-f4dc-bbd5870c7300");
const EFI_DT_APPLY_FIXUPS: u32 = 0x0000_0001;
const EFI_DT_RESERVE_MEMORY: u32 = 0x0000_0002;

struct Devicetree {
    size: usize,
    ptr: NonNull<u8>,
}

impl Devicetree {
    fn new(content: &[u8], size: Option<usize>) -> uefi::Result<Self> {
        let size = size.unwrap_or(content.len());
        let ptr = boot::allocate_pool(boot::MemoryType::ACPI_RECLAIM, size)?;
        unsafe {
            // SAFETY: ptr is exactly the same length as size, so this is safe
            copy_nonoverlapping(content.as_ptr(), ptr.as_ptr(), content.len());
        }
        Ok(Self { size, ptr })
    }
}

impl Drop for Devicetree {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: if the devicetree is out of scope, it will not be used again, so this is safe
            // this will only error if the ptr is invalid (such as if it wasn't allocated by allocate_pool)
            let _ = boot::free_pool(self.ptr);
        }
    }
}

// Lets the firmware apply fixups to the provided devicetree.
fn fixup_devicetree(devicetree: &mut Devicetree) -> uefi::Result<()> {
    let Ok(fixup) = boot::locate_handle_buffer(boot::SearchType::ByProtocol(&DTB_FIXUP_TABLE))
    else {
        return Ok(()); // do nothing if the firmware does not offer fixups
    };

    let Some(fixup) = fixup.first() else {
        return Err(uefi::Status::NOT_FOUND.into()); // this shouldnt happen in any case
    };

    let mut fixup = boot::open_protocol_exclusive::<DevicetreeFixup>(*fixup)?;

    let devtree_as_slice =
        unsafe { core::slice::from_raw_parts(devicetree.ptr.as_ptr(), devicetree.size) };

    // SAFETY: devicetree ptr is likely not a null pointer, so this is safe
    let res = unsafe {
        fixup.fixup(
            devicetree.ptr.as_ptr().cast::<c_void>(),
            &mut devicetree.size,
            EFI_DT_APPLY_FIXUPS | EFI_DT_RESERVE_MEMORY,
        )
    };
    if res == Status::BUFFER_TOO_SMALL {
        // SAFETY: sizeof ptr is guaranteed to be > old_size, this error is only returned if the buffer is too small
        unsafe {
            *devicetree = Devicetree::new(devtree_as_slice, Some(devicetree.size))?;
            fixup
                .fixup(
                    devicetree.ptr.as_ptr().cast::<c_void>(),
                    &mut devicetree.size,
                    EFI_DT_APPLY_FIXUPS | EFI_DT_RESERVE_MEMORY,
                )
                .to_result()?;
        } // call it again
    }

    Ok(())
}

/// Installs a given devicetree into the FDT DTB table.
///
/// Optionally, if available it calls the firmware's devicetree fixup protocol,
/// so that the firmware may apply fixups to the provided devicetree.
///
/// May return an `Error` if the devicetree path is not valid, the handle does not
/// support [`SimpleFileSystem`], or memory allocation fails. If there is failure
/// anywhere after memory is allocated, then the data is freed.
pub fn install_devicetree(
    devicetree: &str,
    fs: &mut ScopedProtocol<SimpleFileSystem>,
) -> uefi::Result<()> {
    let Ok(path) = CString16::try_from(&*normalize_path(devicetree)) else {
        return Err(Status::OUT_OF_RESOURCES.into());
    };

    let Ok(f) = read(fs, &path) else {
        return Err(Status::INVALID_PARAMETER.into());
    };

    let mut devicetree = ManuallyDrop::new(Devicetree::new(&f, None)?);

    if let Err(e) = fixup_devicetree(&mut devicetree) {
        // SAFETY: we never use devicetree ever again as we return an error, so this is safe
        unsafe {
            ManuallyDrop::drop(&mut devicetree);
        }
        return Err(e);
    }

    unsafe {
        // SAFETY: this should be safe since we never modify or free the pointer after unless it fails
        if let Err(e) = boot::install_configuration_table(
            &DTB_CONF_TABLE,
            devicetree.ptr.as_ptr() as *const c_void,
        ) {
            ManuallyDrop::drop(&mut devicetree); // free the memory, if installing configuration table fails
            return Err(e);
        }
    }

    Ok(())
}
