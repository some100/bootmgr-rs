// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The main application logic.
//!
//! This provides callbacks from the Rust side of the UI, as well
//! as a way to get the UI.

use alloc::{rc::Rc, vec};
use bootmgr_rs_core::{
    boot::BootMgr, config::editor::persist::PersistentConfig, system::helper::locate_protocol,
};
use heapless::mpmc::Q8;
use slint::{ModelRc, ToSharedString};
use uefi::{
    Event, Handle,
    boot::ScopedProtocol,
    proto::console::{gop::GraphicsOutput, text::Input},
};

use crate::{
    MainError,
    editor::Editor,
    input::MouseState,
    ui::{slint_backend::SlintBltPixel, slint_inc::Ui},
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

        let mouse = MouseState::new(boot_mgr.boot_config.fg)?;

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
}
