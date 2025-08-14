//! The main application logic.
//!
//! This provides callbacks from the Rust side of the UI, as well
//! as a way to get the UI.

use alloc::{rc::Rc, vec, vec::Vec};
use bootmgr_rs_core::{
    boot::BootMgr,
    config::{Config, editor::persist::PersistentConfig, parsers::Parsers},
    system::helper::locate_protocol,
};
use bytemuck::TransparentWrapper;
use heapless::mpmc::Q8;
use slint::{
    Image, Model, ModelRc, PhysicalSize, SharedString, ToSharedString,
    platform::{
        WindowEvent,
        software_renderer::{MinimalSoftwareWindow, SoftwareRenderer},
    },
};
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

/// The possible commands that may be pushed through the Slint-Rust queue.
pub enum Command {
    /// Save the changes to a [`Config`] given the fields and index.
    SaveChanges {
        /// The fields that will be saved to the [`Config`].
        fields: ModelRc<(slint::SharedString, slint::SharedString)>,

        /// The index of the [`Config`] that is being saved.
        idx: usize,
    },

    /// Save a persistent [`Config`] to the filesystem.
    SaveConfigToFs(usize),

    /// Remove a persistent [`Config`] from the filesystem.
    RemoveConfigFromFs(usize),

    /// Try to boot an entry.
    TryBoot(usize),

    /// Try to edit an entry.
    TryEdit(usize),
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
    pub events: heapless::Vec<Event, 3>,

    /// The [`App`]'s editor, if included and enabled.
    pub editor: Editor,

    /// The queue of editor changes.
    pub queue: Rc<Q8<Command>>,

    /// Stores the collection of persistently saved [`Config`]s.
    pub persist: PersistentConfig,
}

impl App {
    /// Initialize the state of the [`App`].
    pub fn new() -> Result<Self, MainError> {
        let mut boot_mgr = BootMgr::new()?;
        let persist = PersistentConfig::new()?;
        for config in boot_mgr.list_mut() {
            persist.swap_config_in_persist(config);
        }

        let timeout = boot_mgr.boot_config.timeout;

        let input = locate_protocol::<Input>()?;

        let mouse = MouseState::new()?;

        let gop = locate_protocol::<GraphicsOutput>()?;

        let events = heapless::Vec::new();

        let editor = Editor::new();

        let queue = Rc::new(Q8::new());

        Ok(Self {
            boot_mgr,
            timeout,
            input,
            mouse,
            gop,
            events,
            editor,
            queue,
            persist,
        })
    }

    /// Provides the slint main loop for the [`App`].
    ///
    /// The "super-loop" style of UI is used here, since it is overall more aligned with
    /// the other applications. Once it is finished, it will return a [`Handle`] to a loaded application.
    ///
    /// This also handles the graphical drawing. It maintains a framebuffer the size of the screen, and blits those pixels
    /// onto the screen every frame. This is not hardware accelerated, mainly because GOP is the only available, truly cross
    /// platform firmware protocol that we have. For the purposes of rendering a graphical boot picker UI, hardware
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

        self.setup_callbacks(&ui);

        let mut skip_wait = false;
        let handle = || -> Result<Option<Handle>, MainError> {
            loop {
                slint::platform::update_timers_and_animations();

                self.create_events();

                self.handle_input_events(&window)?;

                window.draw_if_needed(|renderer| self.draw_frame(renderer, &mut fb, w, h));

                while let Some(message) = self.queue.dequeue() {
                    match message {
                        Command::SaveChanges { fields, idx } => {
                            let config = self.boot_mgr.get_config(idx);
                            self.editor.save_config(config, &fields);
                            Self::refresh_boot_items(&self.boot_mgr, &ui);
                        }
                        Command::SaveConfigToFs(idx) => {
                            let config = self.boot_mgr.get_config(idx);
                            if !self.persist.contains(config) {
                                self.persist.add_config_to_persist(config);
                            }
                            let _ = self.persist.save_to_fs();
                        }
                        Command::RemoveConfigFromFs(idx) => {
                            let config = self.boot_mgr.get_config(idx);
                            self.persist.remove_config_from_persist(config);
                            let _ = self.persist.save_to_fs();
                        }
                        Command::TryBoot(idx) => {
                            if let Some(handle) = self.maybe_boot(idx, &ui) {
                                return Ok(Some(handle));
                            }
                            skip_wait = true; // skip wait so that state changes take place immediately
                        }
                        Command::TryEdit(idx) => {
                            let config = self.boot_mgr.get_config(idx);
                            self.editor.load_config(config);

                            ui.invoke_fill_fields(self.editor.get_fields());
                            skip_wait = true;
                        }
                    }
                }

                if !window.has_active_animations() && !skip_wait {
                    let duration = slint::platform::duration_until_next_timer_update();
                    self.wait_for_events(duration)?; // try to go to sleep, until a key press, mouse move, or after the duration
                } else if skip_wait {
                    skip_wait = false;
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

    /// Set up the interactions between Slint and Rust.
    ///
    /// The UI and the main loop communicate through a [`Command`] queue, where changes
    /// such as trying to boot an entry, saving changes, or persisting entries are sent one
    /// way to the main loop.
    ///
    /// This message-passing architecture is used over simply wrapping [`BootMgr`] and [`Editor`] in
    /// `Rc<RefCell>`s, due to the cleaner separation between UI and main loop.
    fn setup_callbacks(&self, ui: &Ui) {
        let tx = Rc::downgrade(&self.queue);
        ui.on_save_changes(move |fields, idx| {
            if let Some(tx) = tx.upgrade()
                && let Ok(idx) = usize::try_from(idx)
            {
                let _ = tx.enqueue(Command::SaveChanges { fields, idx });
            }
        });

        let tx = Rc::downgrade(&self.queue);
        ui.on_persist_config(move |idx| {
            if let Some(tx) = tx.upgrade()
                && let Ok(idx) = usize::try_from(idx)
            {
                let _ = tx.enqueue(Command::SaveConfigToFs(idx));
            }
        });

        let tx = Rc::downgrade(&self.queue);
        ui.on_remove_config(move |idx| {
            if let Some(tx) = tx.upgrade()
                && let Ok(idx) = usize::try_from(idx)
            {
                let _ = tx.enqueue(Command::RemoveConfigFromFs(idx));
            }
        });

        let tx = Rc::downgrade(&self.queue);
        ui.on_try_boot(move |idx| {
            if let Some(tx) = tx.upgrade()
                && let Ok(idx) = usize::try_from(idx)
            {
                let _ = tx.enqueue(Command::TryBoot(idx));
            }
        });

        let tx = Rc::downgrade(&self.queue);
        ui.on_try_edit(move |idx| {
            if let Some(tx) = tx.upgrade()
                && let Ok(idx) = usize::try_from(idx)
            {
                let _ = tx.enqueue(Command::TryEdit(idx));
            }
        });
    }

    /// Handle any input events that may have occurred.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the key event could not be dispatched to the window.
    fn handle_input_events(&mut self, window: &Rc<MinimalSoftwareWindow>) -> Result<(), MainError> {
        while let Some(key) = self.handle_key() {
            let str = SharedString::from(key);
            window
                .try_dispatch_event(WindowEvent::KeyPressed { text: str.clone() }) // clones with SharedString are cheap
                .map_err(MainError::SlintError)?;
            window
                .try_dispatch_event(WindowEvent::KeyReleased { text: str })
                .map_err(MainError::SlintError)?;
        }

        while let Some((position, button)) = self.mouse.get_state() {
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

        Ok(())
    }

    /// Might try to boot the currently selected boot option, probably. Will return a handle to the loaded image
    /// if the image is loaded.
    ///
    /// This will return [`None`] if the image could not be loaded.
    fn maybe_boot(&mut self, idx: usize, ui: &Ui) -> Option<Handle> {
        match self.boot_mgr.load(idx) {
            Ok(handle) => Some(handle),
            Err(e) => {
                ui.invoke_display_err(e.to_shared_string());
                self.timeout = -1;
                Self::refresh_boot_items(&self.boot_mgr, ui);
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
