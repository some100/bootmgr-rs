//! The main application logic.
//!
//! This provides callbacks from the Rust side of the UI, as well
//! as a way to get the UI.

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]

use core::cell::RefCell;

use alloc::{rc::Rc, vec::Vec};
use bootmgr_rs_core::{
    boot::BootMgr,
    config::{Config, parsers::Parsers},
};
use slint::{Image, Model, ModelRc, SharedString, VecModel};

use crate::{
    MainError,
    ui::{Ui, create_window, ueficolor_to_slintcolor},
};

/// The main application logic of the bootloader.
pub struct App {
    /// The internal manager of `Config` files.
    pub boot_mgr: Rc<RefCell<BootMgr>>,

    /// The slint user interface.
    pub ui: Ui,
}

impl App {
    /// Initialize the state of the [`App`].
    pub fn new() -> Result<Self, MainError> {
        let boot_mgr = Rc::new(RefCell::new(BootMgr::new()?));

        let list_idx = boot_mgr.borrow_mut().get_default();

        let timeout = boot_mgr.borrow_mut().boot_config.timeout;

        let ui = Self::get_ui(&boot_mgr.borrow(), list_idx, timeout)?;

        Ok(Self {
            boot_mgr,
            ui,
        })
    }

    /// Get an instance of the slint UI.
    #[allow(clippy::similar_names)]
    pub fn get_ui(boot_mgr: &BootMgr, list_idx: usize, timeout: i64) -> Result<Ui, MainError> {
        let (_window, ui) = create_window()?;

        let images = ui.get_images();

        let items: Vec<_> = boot_mgr
            .list()
            .iter()
            .enumerate()
            .map(|(i, config)| {
                (
                    choose_image(&images, config),
                    config.get_preferred_title(Some(i)).into(),
                )
            })
            .collect();

        let items_rc = Rc::new(VecModel::from(items));
        let boot_items = ModelRc::from(items_rc.clone());

        let boot_config = &boot_mgr.boot_config;
        let (fg, bg, highlight_fg, highlight_bg) = (
            ueficolor_to_slintcolor(boot_config.fg),
            ueficolor_to_slintcolor(boot_config.bg),
            ueficolor_to_slintcolor(boot_config.highlight_fg),
            ueficolor_to_slintcolor(boot_config.highlight_bg),
        );

        ui.set_fg(fg);
        ui.set_bg(bg);
        ui.set_highlight_fg(highlight_fg);
        ui.set_highlight_bg(highlight_bg);

        ui.set_items(boot_items.clone());
        ui.set_listIdx(list_idx as i32);
        ui.set_timeout(timeout as i32);

        Ok(ui)
    }
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
