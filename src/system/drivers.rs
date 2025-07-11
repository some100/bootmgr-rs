use alloc::{format, vec::Vec};
use log::error;
use uefi::{
    CStr16, boot, cstr16,
    fs::FileSystem,
    proto::{device_path::DevicePath, loaded_image::LoadedImage, media::file::FileInfo},
};

use crate::{
    error::BootError,
    system::helper::{get_device_path, get_path_cstr, read_filtered_dir},
};

const DRIVER_PREFIX: &CStr16 = cstr16!("\\EFI\\BOOT\\drivers");

fn load_driver(file: &FileInfo, vec: &mut Vec<u8>) -> Result<(), BootError> {
    let handle_path = boot::open_protocol_exclusive::<DevicePath>(boot::image_handle())?;
    let path_cstr = get_path_cstr(DRIVER_PREFIX, file.file_name());
    let path = get_device_path(&handle_path, path_cstr, vec)?;

    let src = boot::LoadImageSource::FromDevicePath {
        device_path: &path,
        boot_policy: uefi::proto::BootPolicy::ExactMatch,
    };
    let handle = boot::load_image(boot::image_handle(), src)?;

    let image = boot::open_protocol_exclusive::<LoadedImage>(handle)?;

    if image.code_type() != boot::MemoryType::BOOT_SERVICES_CODE
        && image.code_type() != boot::MemoryType::RUNTIME_SERVICES_CODE
    {
        return Err(BootError::GenericOwned(format!(
            "File {} is not a driver",
            file.file_name()
        )));
    }

    Ok(boot::start_image(handle)?)
}

pub fn load_drivers() -> Result<(), BootError> {
    let mut fs = FileSystem::new(boot::get_image_file_system(boot::image_handle())?);
    let dir = read_filtered_dir(&mut fs, DRIVER_PREFIX, ".efi");

    let mut vec = Vec::new();
    let mut drivers_loaded = 0;

    for file in dir {
        if let Err(e) = load_driver(&file, &mut vec) {
            error!("{e}");
            continue;
        }

        drivers_loaded += 1;
    }
    if drivers_loaded > 0 {
        reconnect_drivers()?;
    }
    Ok(())
}

fn reconnect_drivers() -> uefi::Result<()> {
    let handles = boot::locate_handle_buffer(boot::SearchType::AllHandles)?;
    for handle in handles.iter() {
        let _ = boot::connect_controller(*handle, None, None, true);
    }
    Ok(())
}
