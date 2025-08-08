//! The main application logic.
//!
//! This provides callbacks from the Rust side of the UI, as well
//! as a way to get the UI.

use core::cell::Cell;

use alloc::{rc::Rc, vec, vec::Vec};
use bootmgr_rs_core::{
    boot::BootMgr,
    config::{Config, parsers::Parsers},
    error::BootError,
};
use slint::{
    Image, Model, ModelRc, PhysicalSize, SharedString, VecModel,
    platform::{WindowEvent, software_renderer::MinimalSoftwareWindow},
};
use uefi::{
    Handle,
    boot::{self, ScopedProtocol},
    proto::console::{
        gop::{BltOp, BltPixel, BltRegion, GraphicsOutput},
        text::Input,
    },
};

use crate::{
    MainError,
    input::MouseState,
    ui::{SlintBltPixel, Ui, create_window, ueficolor_to_slintcolor},
};

/// The current status of the [`App`].
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum AppState {
    /// The app is currently booting an image.
    Booting,

    /// The app is currently running in its main loop.
    #[default]
    Running,
}

#[derive(Clone, Copy, Default)]
pub struct SharedState {
    /// The index, or the currently selected item.
    pub idx: usize,

    /// The current state of the [`App`].
    pub state: AppState,
}

/// The main application logic of the bootloader.
pub struct App {
    /// The internal manager of `Config` files.
    pub boot_mgr: BootMgr,

    /// The timeout before the default boot entry is selected.
    pub timeout: i64,

    /// The [`Input`] of the application.
    pub input: ScopedProtocol<Input>,

    /// The [`MouseState`] of the cursor.
    pub mouse: MouseState,

    /// The [`GraphicsOutput`] of the application.
    pub gop: ScopedProtocol<GraphicsOutput>,

    /// The shared state between the UI and the Rust backend.
    pub shared: Rc<Cell<SharedState>>,
}

impl App {
    /// Initialize the state of the [`App`].
    pub fn new() -> Result<Self, MainError> {
        let boot_mgr = BootMgr::new()?;

        let timeout = boot_mgr.boot_config.timeout;

        let handle = boot::get_handle_for_protocol::<Input>().map_err(BootError::Uefi)?;
        let input = boot::open_protocol_exclusive::<Input>(handle).map_err(BootError::Uefi)?;

        let mouse = MouseState::new()?;

        let handle = boot::get_handle_for_protocol::<GraphicsOutput>().map_err(BootError::Uefi)?;
        let gop =
            boot::open_protocol_exclusive::<GraphicsOutput>(handle).map_err(BootError::Uefi)?;

        // All of this awkward Rc<Cell<T>> wrapping is so that these properties are shared with
        // slint in callbacks.
        let shared = Rc::new(Cell::new(SharedState::default()));

        Ok(Self {
            boot_mgr,
            timeout,
            input,
            mouse,
            gop,
            shared,
        })
    }

    /// Provides the slint main loop for the [`App`].
    ///
    /// The "super-loop" style of UI is used here, since it is overall more aligned with
    /// the other applications. Once it is finished, it will return a [`Handle`] to a loaded application.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the state of the keyboard could not be successfully communicated to the slint Window,
    /// such as if `try_dispatch_event` failed. Error handling isn't too useful here, as it will simply result in a
    /// reboot on key press. Additionally, if there was an error loading an image, it will result in simply exiting the
    /// application.
    pub fn run(&mut self) -> Result<Option<Handle>, MainError> {
        let (w, h) = self.gop.current_mode_info().resolution();

        let (window, ui) = self.get_a_ui(w, h)?;
        let mut fb = vec![SlintBltPixel::new(); w * h];

        let shared_weak = Rc::downgrade(&self.shared);
        ui.on_tryboot(move |x| {
            if let Some(shared) = shared_weak.upgrade() {
                shared.update(|mut shared| {
                    shared.idx = usize::try_from(x).unwrap_or(0);
                    shared.state = AppState::Booting;
                    shared
                });
            }
        });

        loop {
            slint::platform::update_timers_and_animations();

            if let Some(key) = self.handle_key() {
                let str = SharedString::from(key);
                window
                    .try_dispatch_event(WindowEvent::KeyPressed { text: str.clone() })
                    .map_err(MainError::SlintError)?;
                window
                    .try_dispatch_event(WindowEvent::KeyReleased { text: str })
                    .map_err(MainError::SlintError)?;
            }

            if let Some((position, button)) = self.mouse.get_state() {
                window
                    .try_dispatch_event(WindowEvent::PointerMoved { position })
                    .map_err(MainError::SlintError)?;
                window
                    .try_dispatch_event(WindowEvent::PointerPressed { position, button })
                    .map_err(MainError::SlintError)?;

                // normally this would be really bad, however it does not matter in a uefi bootloader where complex mouse
                // button usage is not required
                window
                    .try_dispatch_event(WindowEvent::PointerReleased { position, button })
                    .map_err(MainError::SlintError)?;

                window.request_redraw();
            }

            window.draw_if_needed(|renderer| {
                renderer.render(&mut fb, w);

                // SAFETY: fb is guaranteed nonnull, slintbltpixel is a repr(transparent) type of bltpixel,
                // and len is guaranteed to be the same as the actual len
                let blt_fb = unsafe {
                    core::slice::from_raw_parts_mut(fb.as_mut_ptr().cast::<BltPixel>(), fb.len())
                };

                self.mouse.draw_cursor(blt_fb, w, h);

                let _ = self.gop.blt(BltOp::BufferToVideo {
                    buffer: blt_fb,
                    src: BltRegion::Full,
                    dest: (0, 0),
                    dims: (w, h),
                });
            });

            match self.shared.get().state {
                AppState::Booting => break Ok(self.maybe_boot()),
                AppState::Running => (),
            }
        }
    }

    /// Might try to boot the currently selected boot option, probably. Will return a handle to the loaded image
    /// if the image is loaded.
    ///
    /// This will return [`None`] if the image could not be loaded. In the context of the main loop, this will
    /// essentially result in the application exiting, or shutting down.
    fn maybe_boot(&mut self) -> Option<Handle> {
        self.boot_mgr.load(self.shared.get().idx).ok()
    }

    /// Get an instance of the Slint UI.
    ///
    /// This will set up all the necessary parameters and callbacks needed for the application to run with the
    /// user interface. First, it sets the size of the window to the size parameters (which will usually be the GOP mode).
    /// Then, it gets the images from the UI, and, for each [`Config`] in the [`BootMgr`], it will assign an image
    /// given the origin of the [`Config`], then put those items back into the UI. Then theme settings from `BootConfig`
    /// are applied. Finally, the list index and timeout are set to the application's values.
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

        // this will return a list of every image and its associated parser, such as (img, bls).
        let images = ui.get_images();

        let items: Vec<_> = self
            .boot_mgr
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

        // slint requires that they be in ModelRc, for some reason
        let items_rc = Rc::new(VecModel::from(items));
        let boot_items = ModelRc::from(items_rc.clone());

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
        ui.set_items(boot_items.clone());
        ui.set_listIdx(i32::try_from(self.shared.get().idx).unwrap_or(0));
        ui.set_timeout(i32::try_from(self.timeout).unwrap_or(-1));

        Ok((window, ui))
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
