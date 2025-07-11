use alloc::{string::String, vec::Vec};
use log::error;
use ratatui_core::terminal::Terminal;
use ratatui_widgets::list::ListState;
use uefi::{
    Event, Handle,
    boot::{self, ScopedProtocol},
    proto::console::text::{Input, Key, ScanCode},
};

use crate::{
    boot::BootMgr, error::BootError, parsers::Config, system::drivers::load_drivers,
    ui::ratatui_backend::UefiBackend,
};

pub struct App {
    pub boot_mgr: BootMgr,
    pub boot_options: BootList,
    pub events: Option<[Event; 2]>,
    pub input: ScopedProtocol<Input>,
    pub timeout: i64,
    pub should_boot: bool,
    pub should_exit: bool,
    pub set_default: bool,
}

pub struct BootList {
    pub items: Vec<String>,
    pub state: ListState,
}

impl FromIterator<Config> for BootList {
    fn from_iter<I: IntoIterator<Item = Config>>(iter: I) -> Self {
        let items = iter
            .into_iter()
            .map(|config| config.title.unwrap_or(config.filename))
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

impl App {
    pub fn new() -> Result<Self, BootError> {
        load_drivers()?;

        let boot_mgr = BootMgr::new()?;

        let mut boot_options = BootList::from_iter(boot_mgr.list());
        boot_options.state.select(Some(boot_mgr.get_default()));

        let handle = boot::get_handle_for_protocol::<Input>()?;
        let input = boot::open_protocol_exclusive::<Input>(handle)?;
        Ok(Self {
            boot_mgr,
            boot_options,
            events: None,
            input,
            timeout: 5,
            should_exit: false,
            should_boot: false,
            set_default: false,
        })
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<UefiBackend>,
    ) -> Result<Option<Handle>, BootError> {
        terminal.clear()?;

        let timer_event = self.get_timer_event()?;
        self.events = unsafe {
            Some([
                self.input.wait_for_key_event().unwrap(),
                timer_event.unsafe_clone(),
            ])
        };

        let handle = loop {
            terminal.draw(|f| f.render_widget(&mut *self, f.area()))?;

            self.wait_for_events();

            self.handle_key()?;
            if self.should_boot {
                if let Some(option) = self.boot_options.state.selected() {
                    if self.set_default {
                        self.boot_mgr.set_default(option);
                    }
                    match self.boot_mgr.load(option) {
                        Ok(handle) => break handle,
                        Err(e) => {
                            error!("{e}");
                            boot::stall(5_000_000); // wait for 5 seconds so the error is visible
                            self.timeout = -1;
                            self.should_boot = false;
                            terminal.clear()?; // clear screen so we dont have a messed up terminal
                        }
                    };
                }
            }
            if self.should_exit {
                return Ok(None);
            }
        };
        Ok(Some(handle))
    }

    fn wait_for_events(&mut self) {
        let Some(events) = &mut self.events else {
            return; // if there are somehow no events, dont wait
        };

        if let Ok(i) = boot::wait_for_event(events) {
            if i == 1 {
                self.timeout = self.timeout.saturating_sub(1);
                if self.timeout == 0 {
                    self.should_boot = true;
                }
            }
            return;
        }
    }

    fn handle_key(&mut self) -> Result<(), BootError> {
        match self.input.read_key()? {
            Some(Key::Special(key)) => match key {
                ScanCode::UP => {
                    self.boot_options.state.select_previous();
                    self.timeout = -1;
                }
                ScanCode::DOWN => {
                    self.boot_options.state.select_next();
                    self.timeout = -1;
                }
                ScanCode::ESCAPE => self.should_exit = true,
                _ => (),
            },
            Some(Key::Printable(key)) => {
                let key = char::from(key).to_ascii_lowercase();
                match key {
                    'w' => {
                        self.boot_options.state.select_previous();
                        self.timeout = -1;
                    }
                    's' => {
                        self.boot_options.state.select_next();
                        self.timeout = -1;
                    }
                    '+' | '=' => self.set_default = !self.set_default,
                    '\r' => self.should_boot = true, // return key
                    _ => (),
                }
                self.timeout = -1;
            }
            _ => (),
        }
        Ok(())
    }

    fn get_timer_event(&self) -> Result<Event, BootError> {
        // there are no callbacks, so this is safe
        let timer_event = unsafe {
            boot::create_event(boot::EventType::TIMER, boot::Tpl::APPLICATION, None, None)?
        };
        boot::set_timer(&timer_event, boot::TimerTrigger::Periodic(10_000_000))?;
        Ok(timer_event)
    }
}
