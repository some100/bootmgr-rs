use crate::{
    boot::{
        action::{BootAction, firmware::reset_to_firmware},
        devicetree::install_devicetree,
    },
    error::BootError,
    parsers::Config,
    system::helper::get_device_path,
};

use alloc::vec::Vec;
use log::error;
use uefi::{
    CString16, Handle, Status,
    boot::{self, image_handle},
    proto::{device_path::DevicePath, loaded_image::LoadedImage},
    runtime::{self, ResetType},
};

pub fn load_boot_option(config: &Config) -> Result<Handle, BootError> {
    match config.action {
        BootAction::Reboot => runtime::reset(ResetType::WARM, Status::SUCCESS, None),
        BootAction::Shutdown => runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None),
        BootAction::ResetToFirmware => reset_to_firmware()?,
        BootAction::Boot => (),
    }

    let handle = config
        .handle
        .ok_or(BootError::Generic("No filesystem in config"))?;
    let dev_path = boot::open_protocol_exclusive::<DevicePath>(handle)?;

    let file = config
        .linux
        .as_ref()
        .or(config.efi.as_ref())
        .ok_or(BootError::Generic(
            "Neither linux nor efi key specified in config",
        ))?
        .replace("/", "\\"); // replace forward slash with expected backslash
    let s = CString16::try_from(&*file)?;

    let mut vec = Vec::new();
    let path = get_device_path(&dev_path, s, &mut vec)?;

    let src = boot::LoadImageSource::FromDevicePath {
        device_path: &path,
        boot_policy: uefi::proto::BootPolicy::ExactMatch,
    };
    let handle = boot::load_image(image_handle(), src);

    match handle {
        Err(e) => {
            error!("Error loading image: {e}");
            Err(BootError::Uefi(e))
        }
        Ok(handle) => {
            if let Some(devicetree) = &config.devicetree {
                install_devicetree(devicetree, handle)?;
            }

            let options = config.options.as_ref().map_or("", |v| v);
            let mut image = boot::open_protocol_exclusive::<LoadedImage>(handle)?;
            let load_options = CString16::try_from(&*options)?;
            let load_options_ptr = load_options.as_ptr() as *const u8;
            let load_options_size = load_options.num_bytes() as u32;

            core::mem::forget(load_options); // leak it so that it lasts long enough

            unsafe {
                // this is safe since we already leaked load_options
                image.set_load_options(load_options_ptr, load_options_size);
            }
            Ok(handle)
        }
    }
}
