//! The main application logic.
//!
//! This provides callbacks from the Rust side of the UI, as well
//! as a way to get the UI.

use core::cell::RefCell;

use alloc::{rc::Rc, vec, vec::Vec};
use bootmgr_rs_core::{
    boot::BootMgr,
    config::{Config, parsers::Parsers},
    system::helper::locate_protocol,
};
use bytemuck::TransparentWrapper;
use slint::{
    ComponentHandle, Image, Model, ModelRc, PhysicalSize, SharedString, ToSharedString,
    platform::{
        WindowEvent,
        software_renderer::{MinimalSoftwareWindow, SoftwareRenderer},
    },
};
use smallvec::SmallVec;
use uefi::{
    Event, Handle,
    boot::ScopedProtocol,
    proto::console::{
        gop::{BltOp, BltRegion, GraphicsOutput},
        text::Input,
    },
};

use crate::{
    MainError,
    editor::Editor,
    input::MouseState,
    ui::{SlintBltPixel, create_window, slint_inc::Ui, ueficolor_to_slintcolor},
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
    pub boot_mgr: Rc<RefCell<BootMgr>>,

    /// The timeout before the default boot entry is selected.
    pub timeout: i64,

    /// The [`Input`] of the application.
    pub input: ScopedProtocol<Input>,

    /// The [`MouseState`] of the cursor.
    pub mouse: MouseState,

    /// The [`GraphicsOutput`] of the application.
    pub gop: ScopedProtocol<GraphicsOutput>,

    /// The input events of the system.
    pub events: SmallVec<[Event; 3]>,

    /// The index, or the currently selected item.
    pub idx: usize,

    /// The current state of the [`App`].
    pub state: AppState,

    /// The [`App`]'s editor, if included and enabled.
    pub editor: Rc<RefCell<Editor>>,
}

impl App {
    /// Initialize the state of the [`App`].
    pub fn new() -> Result<Self, MainError> {
        let boot_mgr = BootMgr::new()?;

        let timeout = boot_mgr.boot_config.timeout;

        let input = locate_protocol::<Input>()?;

        let mouse = MouseState::new()?;

        let gop = locate_protocol::<GraphicsOutput>()?;

        let events = SmallVec::new();

        let idx = boot_mgr.get_default();

        let editor = Editor::new();
        Ok(Self {
            boot_mgr: Rc::new(RefCell::new(boot_mgr)),
            timeout,
            input,
            mouse,
            gop,
            events,
            idx,
            state: AppState::Running,
            editor: Rc::new(RefCell::new(editor)),
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
    /// such as if `try_dispatch_event` failed. Additionally, if there was an error loading an image, the error will
    /// be displayed as a popup that can be exited.
    pub fn run(mut self) -> Result<Option<Handle>, MainError> {
        let (w, h) = self.gop.current_mode_info().resolution();

        let (window, ui) = self.get_a_ui(w, h)?;
        let mut fb = vec![SlintBltPixel::new(); w * h];

        let boot_mgr_clone = self.boot_mgr.clone();
        let editor_clone = self.editor.clone();
        let ui_weak = ui.as_weak();
        ui.on_save_changes(move |fields| {
            if let Some(ui) = ui_weak.upgrade() {
                let mut boot_mgr = boot_mgr_clone.borrow_mut();
                let mut editor = editor_clone.borrow_mut();

                let config = boot_mgr.get_config(self.idx);
                editor.save_config(config, &fields);

                Self::refresh_boot_items(&boot_mgr, &ui);
            }
        });

        let handle = || -> Result<Option<Handle>, MainError> {
            loop {
                slint::platform::update_timers_and_animations();

                self.create_events();

                if let Some(key) = self.handle_key() {
                    let str = SharedString::from(key);
                    window
                        .try_dispatch_event(WindowEvent::KeyPressed { text: str.clone() }) // clones with SharedString are cheap
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

                if let Some(handle) = self.maybe_boot(&ui) {
                    break Ok(Some(handle));
                }

                self.maybe_launch_editor(&ui);

                if !window.has_active_animations() {
                    let duration = slint::platform::duration_until_next_timer_update();
                    self.wait_for_events(duration)?; // try to go to sleep, until a key press, mouse move, or after the duration
                }
            }
        }();

        match handle {
            Err(e) => {
                ui.invoke_display_fatal_err(e.to_shared_string());
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
    /// This will return [`None`] if the image could not be loaded.
    fn maybe_boot(&mut self, ui: &Ui) -> Option<Handle> {
        if self.state != AppState::Booting {
            return None;
        }

        let mut boot_mgr = self.boot_mgr.borrow_mut();
        match boot_mgr.load(self.idx) {
            Ok(handle) => Some(handle),
            Err(e) => {
                ui.invoke_display_err(e.to_shared_string());
                self.state = AppState::Running;
                self.timeout = -1;
                Self::refresh_boot_items(&boot_mgr, ui);
                None
            }
        }
    }

    /// Get an instance of the Slint UI.
    ///
    /// This will set up all the necessary parameters and callbacks needed for the application to run with the
    /// user interface. First, it sets the size of the window to the size parameters (which will usually be the GOP mode).
    /// Then, it gets the images from the UI, and, for each [`Config`] in the [`BootMgr`], it will assign an image
    /// given the origin of the [`Config`], then put those items back into the UI. Then theme settings from `BootConfig`
    /// are applied. Finally, the list index and timeout are set to the application's values.
    fn get_a_ui(&self, w: usize, h: usize) -> Result<(Rc<MinimalSoftwareWindow>, Ui), MainError> {
        let (window, ui) = create_window()?;
        window.set_size(PhysicalSize::new(
            u32::try_from(w).unwrap_or(0),
            u32::try_from(h).unwrap_or(0),
        ));

        Self::refresh_boot_items(&self.boot_mgr.borrow(), &ui);

        // applying theme
        let boot_config = &self.boot_mgr.borrow().boot_config;
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
        ui.set_listIdx(i32::try_from(self.idx).unwrap_or(0));
        ui.set_timeout(i32::try_from(self.timeout).unwrap_or(-1));

        // this has to be explicitly set as false for some reason?
        ui.set_now_editing(false);

        Ok((window, ui))
    }

    /// Draws a frame to the screen.
    fn draw_frame(
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

    /// Launch the editor.
    fn maybe_launch_editor(&mut self, ui: &Ui) {
        let mut boot_mgr = self.boot_mgr.borrow_mut();
        let mut editor = self.editor.borrow_mut();
        if ui.get_now_editing() && !editor.editing {
            let config = boot_mgr.get_config(self.idx);
            editor.load_config(config);

            ui.invoke_fill_fields(editor.get_fields());
        } else if editor.editing {
            editor.editing = false;
        }
    }

    /// Refresh the available boot items given the list of configurations.
    fn refresh_boot_items(boot_mgr: &BootMgr, ui: &Ui) {
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
