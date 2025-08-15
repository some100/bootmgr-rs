//! The main application logic.
//!
//! This is where the main loop of the whole application is located, and is where terminal, boot manager,
//! and editor interact.

use bootmgr_rs_core::{
    boot::BootMgr,
    config::editor::persist::PersistentConfig,
    error::BootError,
    system::helper::{create_timer, locate_protocol},
};
use log::error;
use ratatui_core::terminal::Terminal;
use thiserror::Error;
use uefi::{
    Event, Handle,
    boot::{self, ScopedProtocol, TimerTrigger},
    proto::console::text::{Input, Key, ScanCode},
};

use crate::{
    MainError,
    editor::EditorState,
    ui::{boot_list::BootList, ratatui_backend::UefiBackend, theme::Theme},
};

use crate::features::editor::Editor;

/// The error delay in microseconds.
const ERROR_DELAY: usize = 5_000_000; // 5 seconds

/// The timeout timer interval in microseconds.
const TIMER_INTERVAL: u64 = 10_000_000; // 1 second

/// An `Error` that may result from running or initializing the [`App`].
#[derive(Error, Debug)]
pub enum AppError {
    /// The [`Input`] protocol was closed for any reason.
    #[error("Keyboard Input protocol was closed")]
    InputClosed,

    /// There are no boot entries in the boot list.
    #[error("No boot entries found")]
    NoEntries,
}

/// The current status of the [`App`].
#[derive(PartialEq, Eq)]
pub enum AppState {
    /// The app is currently booting an image.
    Booting,

    /// The app is currently running in its main loop.
    Running,

    /// The app is currently exiting.
    Exiting,
}

/// The main application logic of the bootloader.
pub struct App {
    /// The internal manager of `Config` files.
    pub boot_mgr: BootMgr,

    /// The list of boot names.
    pub boot_list: BootList,

    /// The storage for the `wait_for_key` events and the timer event.
    pub events: Option<[Event; 2]>,

    /// The [`Input`] of the terminal.
    pub input: ScopedProtocol<Input>,

    /// The [`Theme`] of the UI.
    pub theme: Theme,

    /// The timeout before the default boot entry is selected.
    pub timeout: i64,

    /// Checks if a default boot option is being selected.
    pub set_default: bool,

    /// The current state of the [`App`].
    pub state: AppState,

    /// The [`App`]'s editor, if included and enabled.
    pub editor: Editor,
}

impl App {
    /// Initializes the state of the [`App`].
    ///
    /// This parses configuration, and finds a [`Handle`] for [`Input`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the [`BootMgr`] could not be created, or there is no [`Handle`] supporting
    /// [`Input`]
    pub fn new() -> Result<Self, MainError> {
        let mut boot_mgr = BootMgr::new()?;

        let persist = PersistentConfig::new()?;
        for config in boot_mgr.list_mut() {
            persist.swap_config_in_persist(config);
        }

        let boot_list = BootList::new(&boot_mgr);

        if boot_list.items.is_empty() {
            return Err(AppError::NoEntries.into());
        }

        let theme = Theme::new(&boot_mgr.boot_config);

        let timeout = boot_mgr.boot_config.timeout;

        let input = locate_protocol::<Input>()?;

        let editor = Editor::new(&input, theme, persist)?;
        Ok(Self {
            boot_mgr,
            boot_list,
            events: None,
            input,
            theme,
            timeout,
            set_default: false,
            state: AppState::Running,
            editor,
        })
    }

    /// Provides the main loop for the [`App`]
    ///
    /// This is where the UI and key handling are centrally managed, and if enabled
    /// it can also hand off control to the [`Editor`]. When a boot option is loaded,
    /// it will return with a [`Handle`] to that loaded image.
    ///
    /// # Errors
    ///
    /// May return an `Error` if a frame could not be drawn, the input was closed,
    /// or the editor failed to run if enabled.
    pub fn run(
        &mut self,
        terminal: &mut Terminal<UefiBackend>,
    ) -> Result<Option<Handle>, MainError> {
        self.init_state(terminal)?;

        let handle = loop {
            self.draw(terminal)?;

            self.handle_key()?;

            if let Some(handle) = self.maybe_boot(terminal)? {
                break handle;
            }

            if self.state == AppState::Exiting {
                return Ok(None);
            }

            self.maybe_launch_editor(terminal)?;
        };

        Ok(Some(handle))
    }

    /// Initializes the state of the terminal and events.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the terminal could not be cleared, or the events could not be created.
    fn init_state(&mut self, terminal: &mut Terminal<UefiBackend>) -> Result<(), MainError> {
        if let (Some(fg), Some(bg)) = (self.theme.base.fg, self.theme.base.bg) {
            terminal.backend_mut().set_color(fg, bg);
        }
        terminal.clear()?;

        self.create_events()?;

        Ok(())
    }

    /// Might try to boot the currently selected boot option, probably. Will return a handle to the loaded image
    /// if the image is loaded.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the terminal could not be cleared.
    fn maybe_boot(
        &mut self,
        terminal: &mut Terminal<UefiBackend>,
    ) -> Result<Option<Handle>, MainError> {
        if self.state != AppState::Booting {
            return Ok(None);
        }

        let Some(option) = self.boot_list.state.selected() else {
            return Ok(None);
        };

        if self.set_default {
            self.boot_mgr.set_default(option);
        }

        match self.boot_mgr.load(option) {
            Ok(handle) => Ok(Some(handle)),
            Err(e) => {
                terminal.backend_mut().reset_color();
                error!("Failed to load image: {e}");
                boot::stall(ERROR_DELAY); // wait for 5 seconds so the error is visible
                self.timeout = -1;
                self.state = AppState::Running;
                terminal.clear()?; // clear screen so we dont have a messed up terminal
                self.boot_list = BootList::new(&self.boot_mgr);
                Ok(None)
            }
        }
    }

    /// Might launch the editor, probably.
    ///
    /// # Errors
    ///
    /// May return an `Error` if there was some sort of error or failure in the interactive editor.
    fn maybe_launch_editor(
        &mut self,
        terminal: &mut Terminal<UefiBackend>,
    ) -> Result<(), MainError> {
        if self.editor.state == EditorState::Editing
            && self.boot_mgr.boot_config.editor
            && let Some(option) = self.boot_list.state.selected()
        {
            let config = self.boot_mgr.get_config(option);
            self.editor.run(config, &mut self.input, terminal)?;

            self.boot_mgr.validate();
            self.boot_list = BootList::new(&self.boot_mgr);
        }

        Ok(())
    }

    /// Waits for one of the two events, the timeout and key press.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the events could not be created.
    fn wait_for_events(&mut self) -> Result<(), MainError> {
        let Some(events) = &mut self.events else {
            return Ok(()); // if there are somehow no events, dont wait
        };

        if self.timeout == 0 {
            self.state = AppState::Booting;
            return Ok(()); // if timeout is 0, dont wait and try booting immediately
        }

        match boot::wait_for_event(events) {
            Ok(i) => {
                if i == 1 {
                    self.timeout = self.timeout.saturating_sub(1);
                }
            }
            Err(e) => {
                error!("{e}");
                self.events.take();
                self.create_events()?;
            }
        }
        Ok(())
    }

    /// Create the key and timer events.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the input is closed, or the timer event could not be opened.
    fn create_events(&mut self) -> Result<(), MainError> {
        self.events = Some([
            self.input
                .wait_for_key_event()
                .ok_or(AppError::InputClosed)?,
            create_timer(TimerTrigger::Periodic(TIMER_INTERVAL))?,
        ]);
        Ok(())
    }

    /// Wait for a key press, then handle it.
    ///
    /// # Errors
    ///
    /// May return an `Error` if there was some sort of device error with the [`Input`].
    fn handle_key(&mut self) -> Result<(), MainError> {
        self.wait_for_events()?;
        match self.input.read_key().map_err(BootError::Uefi)? {
            Some(Key::Special(key)) => self.handle_special_key(key),
            Some(Key::Printable(key)) => self.handle_printable_key(key.into()),
            _ => (),
        }
        Ok(())
    }

    /// Handle a special key.
    ///
    /// This includes the arrow keys for selection, and the escape key for exiting.
    fn handle_special_key(&mut self, key: ScanCode) {
        match key {
            ScanCode::UP => {
                self.boot_list.state.select_previous();
                self.timeout = -1;
            }
            ScanCode::DOWN => {
                self.boot_list.state.select_next();
                self.timeout = -1;
            }
            ScanCode::ESCAPE => self.state = AppState::Exiting,
            _ => (),
        }
    }

    /// Handle a printable key.
    ///
    /// This includes w/s for alternate selection, +/= for setting the default, e for editing, or the
    /// enter key for selecting a boot option.
    fn handle_printable_key(&mut self, key: char) {
        let key = key.to_ascii_lowercase();
        match key {
            'w' => {
                self.boot_list.state.select_previous();
                self.timeout = -1;
            }
            's' => {
                self.boot_list.state.select_next();
                self.timeout = -1;
            }
            '+' | '=' => self.set_default = !self.set_default,
            '\r' => self.state = AppState::Booting, // return key
            'e' => self.editor.state = EditorState::Editing,
            _ => (),
        }
        self.timeout = -1;
    }
}
