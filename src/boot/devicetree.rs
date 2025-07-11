// WARNING: None of this is actually tested.
// This is mostly based off of systemd-boot's implementation

use core::ffi::c_void;
use core::ptr::copy_nonoverlapping;

use uefi::fs::FileSystem;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{CString16, guid, prelude::*};

use crate::system::protos::DevicetreeFixup;

const DTB_CONF_TABLE: uefi::Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");
const DTB_FIXUP_TABLE: uefi::Guid = guid!("e617d64c-fe08-46da-f4dc-bbd5870c7300");
const EFI_DT_APPLY_FIXUPS: u32 = 0x0000_0001;
const EFI_DT_RESERVE_MEMORY: u32 = 0x0000_0002;

// Lets the firmware apply fixups to the provided devicetree.
fn fixup_devicetree(mut buf: *mut c_void, mut size: usize) -> uefi::Result<*mut c_void> {
    let Ok(fixup) = boot::locate_handle_buffer(boot::SearchType::ByProtocol(&DTB_FIXUP_TABLE))
    else {
        return Ok(buf); // skip it
    };

    let fixup = fixup.first().unwrap(); // if we found a handle, it should be safe to unwrap

    let mut fixup = boot::open_protocol_exclusive::<DevicetreeFixup>(*fixup)?;

    let old_size = size;

    let res = fixup.fixup(buf, &mut size, EFI_DT_APPLY_FIXUPS | EFI_DT_RESERVE_MEMORY);
    if res == Status::BUFFER_TOO_SMALL {
        let ptr =
            boot::allocate_pool(boot::MemoryType::ACPI_RECLAIM, size)?.as_ptr() as *mut c_void;
        unsafe {
            copy_nonoverlapping(buf, ptr, old_size); // should be safe since sizeof ptr > old_size
            buf = ptr;
        }
        let res = fixup.fixup(buf, &mut size, EFI_DT_APPLY_FIXUPS | EFI_DT_RESERVE_MEMORY);
        if res.is_error() {
            return Err(res.into());
        }
    }

    Ok(buf)
}

pub fn install_devicetree(devicetree: &str, handle: Handle) -> uefi::Result<()> {
    let mut fs = FileSystem::new(boot::open_protocol_exclusive::<SimpleFileSystem>(handle)?);

    let Ok(path) = CString16::try_from(&*devicetree.replace("/", "\\")) else {
        return Err(Status::OUT_OF_RESOURCES.into());
    };

    let Ok(f) = fs.read(&*path) else {
        return Err(Status::INVALID_PARAMETER.into());
    };
    let f_len = f.len();

    let f_ptr = boot::allocate_pool(boot::MemoryType::ACPI_RECLAIM, f_len)?.as_ptr() as *mut c_void;
    unsafe {
        core::ptr::copy_nonoverlapping(f.as_ptr(), f_ptr as *mut u8, f_len); // this is totally fine
    }

    let f_ptr = fixup_devicetree(f_ptr, f_len)?;

    unsafe {
        boot::install_configuration_table(&DTB_CONF_TABLE, f_ptr)?;
    }

    Ok(())
}
