//! The main application logic.
//!
//! This provides callbacks from the Rust side of the UI, as well
//! as a way to get the UI.
//!
//! # Safety
//!
//! This uses unsafe blocks in 1 place.
//!
//! 1. [`SlintBltPixel`] is simply a `repr(transparent)` wrapper around [`BltPixel`]. Therefore, the components of this type
//!    are identical and therefore can be safely reinterpreted as [`BltPixel`].

use alloc::{rc::Rc, vec, vec::Vec};
use bootmgr_rs_core::{
    boot::BootMgr,
    config::{Config, parsers::Parsers},
    system::helper::locate_protocol,
};
use slint::{
    Image, Model, ModelRc, PhysicalSize, SharedString, ToSharedString, VecModel,
    platform::{
        WindowEvent,
        software_renderer::{MinimalSoftwareWindow, SoftwareRenderer},
    },
};
use uefi::{
    Event, Handle,
    boot::ScopedProtocol,
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

    /// The input events of the system.
    pub events: Vec<Event>,

    /// The index, or the currently selected item.
    pub idx: usize,

    /// The current state of the [`App`].
    pub state: AppState,
}

impl App {
    /// Initialize the state of the [`App`].
    pub fn new() -> Result<Self, MainError> {
        let boot_mgr = BootMgr::new()?;

        let timeout = boot_mgr.boot_config.timeout;

        let input = locate_protocol::<Input>()?;

        let mouse = MouseState::new()?;

        let gop = locate_protocol::<GraphicsOutput>()?;

        let events = Vec::new();

        let idx = boot_mgr.get_default();

        Ok(Self {
            boot_mgr,
            timeout,
            input,
            mouse,
            gop,
            events,
            idx,
            state: AppState::Running,
        })
    }

    /// Provides the slint main loop for the [`App`].
    ///
    /// The "super-loop" style of UI is used here, since it is overall more aligned with
    /// the other applications. Once it is finished, it will return a [`Handle`] to a loaded application.
    ///
    /// This also handles the graphical drawing. It maintains a framebuffer the size of the screen, and blits those pixels
    /// onto the screen every frame. This is not hardware accelerated, mainly because GOP is the only available, truly cross
    /// platform and firmware protocol that we have. For the purposes of rendering a graphical boot picker UI, hardware
    /// acceleration is not necessary. The animations will still look smooth.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the state of the keyboard could not be successfully communicated to the slint Window,
    /// such as if `try_dispatch_event` failed. Error handling isn't too useful here, as it will simply result in a
    /// reboot on key press. Additionally, if there was an error loading an image, it will result in simply exiting the
    /// application.
    pub fn run(mut self) -> Result<Option<Handle>, MainError> {
        let (w, h) = self.gop.current_mode_info().resolution();

        let (window, ui) = self.get_a_ui(w, h)?;
        let mut fb = vec![SlintBltPixel::new(); w * h];

        let handle = || -> Result<Option<Handle>, MainError> {
            loop {
                slint::platform::update_timers_and_animations();

                self.create_events();

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

                window.draw_if_needed(|renderer| self.draw_frame(renderer, &mut fb, w, h));

                if let Ok(idx) = usize::try_from(ui.get_listIdx())
                    && idx != self.idx
                {
                    self.idx = idx;
                }

                if ui.get_now_booting() {
                    self.state = AppState::Booting;
                }

                match self.state {
                    AppState::Booting => break Ok(self.maybe_boot(self.idx)),
                    AppState::Running => (),
                }

                if !window.has_active_animations() {
                    let duration = slint::platform::duration_until_next_timer_update();
                    self.wait_for_events(duration)?; // try to go to sleep, until a key press, mouse move, or after the duration
                }
            }
        }();

        match handle {
            Err(e) => {
                ui.invoke_display_err(e.to_shared_string());
                window.request_redraw();
                window.draw_if_needed(|renderer| self.draw_frame(renderer, &mut fb, w, h));
                Err(e)
            }
            image => image,
        }
    }

    /// Might try to boot the currently selected boot option, probably. Will return a handle to the loaded image
    /// if the image is loaded.
    ///
    /// This will return [`None`] if the image could not be loaded. In the context of the main loop, this will
    /// essentially result in the application exiting, or shutting down.
    fn maybe_boot(&mut self, idx: usize) -> Option<Handle> {
        self.boot_mgr.load(idx).ok()
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
        ui.set_listIdx(i32::try_from(self.idx).unwrap_or(0));
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
