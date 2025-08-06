//! The main application logic.
//!
//! This provides callbacks from the Rust side of the UI, as well
//! as a way to get the UI.

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]

use alloc::{rc::Rc, string::ToString, vec::Vec};
use bootmgr_rs_core::{
    boot::BootMgr,
    config::{Config, parsers::Parsers},
};
use slint::{Image, Model, ModelRc, SharedString, ToSharedString, VecModel};

use crate::{
    MainError,
    ui::{Ui, create_window},
};

/// The main application logic of the bootloader.
pub struct App {
    /// The internal manager of `Config` files.
    pub boot_mgr: BootMgr,

    /// The index of the currently selected boot option.
    pub list_idx: usize,

    /// The timeout before the default boot entry is selected.
    pub timeout: i64,
}

impl App {
    /// Initialize the state of the [`App`].
    pub fn new() -> Result<Self, MainError> {
        let boot_mgr = BootMgr::new()?;

        let list_idx = boot_mgr.get_default();

        let timeout = boot_mgr.boot_config.timeout;

        Ok(Self {
            boot_mgr,
            list_idx,
            timeout,
        })
    }

    /// Get an instance of the slint UI.
    pub fn get_ui(&mut self) -> Result<Ui, MainError> {
        let (_window, ui) = create_window()?;

        let images = ui.get_images();

        let items: Vec<_> = self
            .boot_mgr
            .list()
            .iter()
            .enumerate()
            .map(|(i, config)| (choose_image(&images, config), choose_title(config, i)))
            .collect();

        let items_rc = Rc::new(VecModel::from(items));
        let boot_items = ModelRc::from(items_rc.clone());

        ui.set_items(boot_items.clone());
        ui.set_listIdx(self.list_idx as i32);
        ui.set_timeout(self.timeout as i32);
        Ok(ui)
    }

    /// Try to boot give an index, and panic if it fails.
    pub fn try_boot(&mut self, idx: i32) {
        let image = self.boot_mgr.load(idx as usize).unwrap();
        uefi::boot::start_image(image).unwrap();
    }
}

/// Choose a title based on if the [`Config`] contains a title, if it contains a non-empty filename, or the index of the boot option
/// as an absolute fallback.
fn choose_title(config: &Config, i: usize) -> SharedString {
    config
        .title
        .clone()
        .unwrap_or_else(|| {
            if config.filename.is_empty() {
                i.to_string()
            } else {
                config.filename.clone()
            }
        })
        .to_shared_string()
}

/// Pick an image based on the origin of the [`Config`].
fn choose_image(images: &ModelRc<(Image, SharedString)>, config: &Config) -> Image {
    let origin = config.origin.map(Parsers::as_str);
    for image in images.iter() {
        if origin == Some(image.1.as_str()) {
            return image.0;
        }
    }
    for image in images.iter() {
        if image.1.as_str() == "fallback" {
            return image.0; // fallback image if the config does not contain an origin
        }
    }
    unreachable!();
}
