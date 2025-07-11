//! The main application logic

use alloc::{string::String, vec::Vec};
use log::error;
use ratatui_core::{style::Color, terminal::Terminal};
use ratatui_widgets::list::ListState;
use uefi::{
    Event, Handle,
    boot::{self, ScopedProtocol},
    proto::console::text::{Input, Key, ScanCode},
};

use crate::{boot::BootMgr, config::Config, error::BootError, ui::ratatui_backend::UefiBackend};

use crate::features::editor::Editor;

const ERROR_DELAY: usize = 5_000_000; // 5 seconds
const TIMER_INTERVAL: u64 = 10_000_000; // 1 second

/// The main application logic of the bootloader.
pub struct App {
    fg: Color,
    bg: Color,
    pub boot_mgr: BootMgr,
    pub boot_list: BootList,
    pub events: Option<[Event; 2]>,
    pub input: ScopedProtocol<Input>,
    pub timeout: i64,
    pub should_boot: bool,
    pub should_exit: bool,
    pub set_default: bool,
    pub editor: Editor,
}

/// The UI frontend for the [`Config`]s.
pub struct BootList {
    pub items: Vec<String>,
    pub state: ListState,
}

impl FromIterator<Config> for BootList {
    fn from_iter<I: IntoIterator<Item = Config>>(iter: I) -> Self {
        let items = iter
            .into_iter()
            .map(|config| config.title.unwrap_or(config.filename)) // if title is nonexistent, use the filename
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

impl BootList {
    /// Creates a new [`BootList`] given a [`BootMgr`].
    ///
    /// This simply creates a [`BootList`] from the inner [`Vec<Config>`] of the [`BootMgr`],
    /// then selects the default option given from the [`BootMgr`].
    #[must_use]
    pub fn new(boot_mgr: &BootMgr) -> Self {
        let mut boot_list = BootList::from_iter(boot_mgr.list());
        boot_list.state.select(Some(boot_mgr.get_default()));
        boot_list
    }
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
    pub fn new() -> Result<Self, BootError> {
        let boot_mgr = BootMgr::new()?;
        let boot_list = BootList::new(&boot_mgr);

        let fg = boot_mgr.boot_config.fg; // store these colors in the main struct for convenience
        let bg = boot_mgr.boot_config.bg;

        let timeout = boot_mgr.boot_config.timeout;

        let handle = boot::get_handle_for_protocol::<Input>()?;
        let input = boot::open_protocol_exclusive::<Input>(handle)?;
        Ok(Self {
            fg,
            bg,
            boot_mgr,
            boot_list,
            events: None,
            input,
            timeout,
            should_exit: false,
            should_boot: false,
            set_default: false,
            editor: Editor::new(),
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
    ) -> Result<Option<Handle>, BootError> {
        self.init_state(terminal)?;

        let handle = loop {
            terminal.draw(|f| f.render_widget(&mut *self, f.area()))?;

            self.wait_for_events()?;

            self.handle_key()?;

            if let Some(handle) = self.maybe_boot(terminal)? {
                break handle;
            }

            if self.should_exit {
                return Ok(None);
            }

            self.maybe_launch_editor(terminal)?;
        };

        Ok(Some(handle))
    }

    /// Drops the [`Terminal`], and consumes `self`.
    pub fn close(self, terminal: Terminal<UefiBackend>) {
        drop(terminal);
    }

    // initializes the state of the terminal and events
    fn init_state(&mut self, terminal: &mut Terminal<UefiBackend>) -> Result<(), BootError> {
        terminal.backend_mut().set_color(self.fg, self.bg);
        terminal.clear()?;

        self.create_events()?;

        Ok(())
    }

    // might try to boot the currently selected boot option, probably
    fn maybe_boot(
        &mut self,
        terminal: &mut Terminal<UefiBackend>,
    ) -> Result<Option<Handle>, BootError> {
        if self.should_boot
            && let Some(option) = self.boot_list.state.selected()
        {
            if self.set_default {
                self.boot_mgr.set_default(option);
            }
            match self.boot_mgr.load(option) {
                Ok(handle) => return Ok(Some(handle)),
                Err(e) => {
                    terminal.backend_mut().reset_color();
                    error!("{e}");
                    boot::stall(ERROR_DELAY); // wait for 5 seconds so the error is visible
                    self.timeout = -1;
                    self.should_boot = false;
                    terminal.clear()?; // clear screen so we dont have a messed up terminal
                }
            }
        }

        Ok(None)
    }

    // might launch the editor, probably
    fn maybe_launch_editor(
        &mut self,
        terminal: &mut Terminal<UefiBackend>,
    ) -> Result<(), BootError> {
        if self.editor.editing
            && self.boot_mgr.boot_config.editor
            && let Some(option) = self.boot_list.state.selected()
        {
            let config = self.boot_mgr.get_config(option);
            self.editor
                .run(config, &mut self.input, terminal, self.fg, self.bg)?;

            self.boot_mgr.validate();
            self.boot_list = BootList::new(&self.boot_mgr);
        }

        Ok(())
    }

    // waits for one of the two events, the timeout and key press
    fn wait_for_events(&mut self) -> Result<(), BootError> {
        let Some(events) = &mut self.events else {
            return Ok(()); // if there are somehow no events, dont wait
        };

        match boot::wait_for_event(events) {
            Ok(i) => {
                if i == 1 {
                    self.timeout = self.timeout.saturating_sub(1);
                    if self.timeout == 0 {
                        self.should_boot = true;
                    }
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

    fn create_events(&mut self) -> Result<(), BootError> {
        let timer_event = Self::get_timer_event()?;
        self.events = unsafe {
            Some([
                self.input
                    .wait_for_key_event()
                    .ok_or(BootError::InputClosed)?,
                timer_event.unsafe_clone(),
            ])
        };
        Ok(())
    }

    fn handle_key(&mut self) -> Result<(), BootError> {
        match self.input.read_key()? {
            Some(Key::Special(key)) => self.handle_special_key(key),
            Some(Key::Printable(key)) => self.handle_printable_key(key.into()),
            _ => (),
        }
        Ok(())
    }

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
            ScanCode::ESCAPE => self.should_exit = true,
            _ => (),
        }
    }

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
            '\r' => self.should_boot = true, // return key
            'e' => self.editor.editing = true,
            _ => (),
        }
        self.timeout = -1;
    }

    fn get_timer_event() -> Result<Event, BootError> {
        // there are no callbacks, so this is safe
        let timer_event = unsafe {
            boot::create_event(boot::EventType::TIMER, boot::Tpl::APPLICATION, None, None)?
        };
        boot::set_timer(&timer_event, boot::TimerTrigger::Periodic(TIMER_INTERVAL))?;
        Ok(timer_event)
    }
}
