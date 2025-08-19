// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The user interface rendering of the Slint bootloader.

use alloc::{rc::Rc, vec::Vec};
use bootmgr::{
    boot::BootMgr,
    config::{Config, parsers::Parsers},
};
use bytemuck::TransparentWrapper;
use slint::{
    Image, Model, ModelRc, PhysicalSize, SharedString,
    platform::software_renderer::{MinimalSoftwareWindow, SoftwareRenderer},
};
use uefi::proto::console::gop::{BltOp, BltRegion};

use crate::{
    MainError,
    app::App,
    ui::{
        slint_backend::{SlintBltPixel, create_window, ueficolor_to_slintcolor},
        slint_inc::Ui,
    },
};

pub mod slint_backend;
pub mod slint_inc;

impl App {
    /// Get an instance of the Slint UI.
    ///
    /// This will set up all the necessary parameters and callbacks needed for the application to run with the
    /// user interface. First, it sets the size of the window to the size parameters (which will usually be the GOP mode).
    /// Then, it gets the images from the UI, and, for each [`Config`] in the [`BootMgr`], it will assign an image
    /// given the origin of the [`Config`], then put those items back into the UI. Then theme settings from `BootConfig`
    /// are applied. Finally, the list index and timeout are set to the application's values.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the window could not be created.
    pub fn get_a_ui(
        &self,
        w: usize,
        h: usize,
    ) -> Result<(Rc<MinimalSoftwareWindow>, Ui), MainError> {
        let (window, ui) = create_window()?;
        window.set_size(PhysicalSize::new(
            u32::try_from(w).unwrap_or(0),
            u32::try_from(h).unwrap_or(0),
        ));

        Self::refresh_boot_items(&self.boot_mgr, &ui);

        // applying theme
        let boot_config = &self.boot_mgr.boot_config;
        let (fg, bg, h_foreground, h_background) = (
            ueficolor_to_slintcolor(boot_config.fg),
            ueficolor_to_slintcolor(boot_config.bg),
            ueficolor_to_slintcolor(boot_config.highlight_fg),
            ueficolor_to_slintcolor(boot_config.highlight_bg),
        );

        ui.set_fg(fg);
        ui.set_bg(bg);
        ui.set_highlight_fg(h_foreground);
        ui.set_highlight_bg(h_background);

        // set up the rest of properties
        ui.set_listIdx(i32::try_from(self.boot_mgr.get_default()).unwrap_or(0));
        ui.set_timeout(i32::try_from(self.timeout).unwrap_or(-1));

        Ok((window, ui))
    }

    /// Draws a frame to the screen.
    pub fn draw_frame(
        &mut self,
        renderer: &SoftwareRenderer,
        fb: &mut [SlintBltPixel],
        w: usize,
        h: usize,
    ) {
        renderer.render(fb, w);

        let blt_fb = TransparentWrapper::peel_slice(fb);

        let _ = self.gop.blt(BltOp::BufferToVideo {
            buffer: blt_fb,
            src: BltRegion::Full,
            dest: (0, 0),
            dims: (w, h),
        });

        if self.mouse.enabled() {
            let _ = self.gop.blt(BltOp::VideoFill {
                color: self.mouse.color(),
                dest: self.mouse.position(),
                dims: self.mouse.dims(),
            });
        }
    }

    /// Refresh the available boot items given the list of configurations.
    pub fn refresh_boot_items(boot_mgr: &BootMgr, ui: &Ui) {
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

        let boot_items = ModelRc::from(&*items);
        ui.set_items(boot_items);
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
