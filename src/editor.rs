#![allow(clippy::cast_possible_truncation)]
//! The optional basic editor for temporarily modifying [`Config`]s

use alloc::{string::String, vec::Vec};
use ratatui_core::{layout::Position, style::Color, terminal::Terminal};
use uefi::{
    Event,
    boot::{self, ScopedProtocol},
    proto::console::text::{Input, Key, ScanCode},
};

use crate::{config::Config, error::BootError, ui::ratatui_backend::UefiBackend};

mod ui;
mod widget;

/// The basic editor
#[derive(Default)]
pub struct Editor {
    pub editing: bool,
    pub events: Option<[Event; 1]>,
    pub cursor_pos: usize,
    pub fields: Vec<(&'static str, String)>,
    pub idx: usize,
    pub value: String,
    pub fg: Color,
    pub bg: Color,
}

impl Editor {
    /// Creates a new [`Editor`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Provides the main loop for the [`Editor`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the terminal could not be cleared, if the terminal could not be drawn,
    /// if the cursor could not be shown, the cursor is out of bounds, or the key could not be read
    /// for some reason. If the Input protocol is closed for some reason, that will also cause an `Error`.
    pub fn run(
        &mut self,
        config: &mut Config,
        input: &mut ScopedProtocol<Input>,
        terminal: &mut Terminal<UefiBackend>,
        fg: Color,
        bg: Color,
    ) -> Result<(), BootError> {
        terminal.backend_mut().set_color(fg, bg);
        self.fg = fg;
        self.bg = bg;

        terminal.clear()?;

        self.init_state(input, config)?;

        loop {
            terminal.draw(|f| f.render_widget(&mut *self, f.area()))?;
            terminal.show_cursor()?;
            terminal.set_cursor_position(Position::new(self.cursor_pos as u16, 3))?; // top bar is ALWAYS 3 length

            self.wait_for_events();
            self.handle_key(input)?;

            if !self.editing {
                break;
            }
        }

        self.save_to_config(config);

        terminal.hide_cursor()?;

        Ok(())
    }

    // Reads the [`Config`] file into the field and initializes the state
    fn init_state(
        &mut self,
        input: &ScopedProtocol<Input>,
        config: &Config,
    ) -> Result<(), BootError> {
        self.events = Some([input.wait_for_key_event().ok_or(BootError::InputClosed)?]);
        self.fields = config
            .get_str_fields()
            .into_iter()
            .map(|(k, v)| (k, v.cloned().unwrap_or_default()))
            .collect();
        self.value = self.fields[0].1.clone();
        self.cursor_pos = self.value.chars().count();
        self.idx = 0;
        Ok(())
    }

    fn wait_for_events(&mut self) {
        let Some(events) = &mut self.events else {
            return;
        };

        let _ = boot::wait_for_event(events);
    }

    fn handle_key(&mut self, input: &mut ScopedProtocol<Input>) -> Result<(), BootError> {
        match input.read_key()? {
            Some(Key::Special(key)) => match key {
                ScanCode::ESCAPE => {
                    self.save_to_field();
                    self.editing = false;
                }
                ScanCode::UP => {
                    self.save_to_field();
                    if self.idx > 0 {
                        self.idx -= 1;
                    }
                    self.load_from_field();
                }
                ScanCode::DOWN => {
                    self.save_to_field();
                    if self.idx + 1 < self.fields.len() {
                        self.idx += 1;
                    }
                    self.load_from_field();
                }
                ScanCode::LEFT => {
                    self.cursor_pos = self.cursor_pos.saturating_sub(1);
                }
                ScanCode::RIGHT => {
                    self.cursor_pos = (self.cursor_pos + 1).min(self.value.len());
                }
                _ => (),
            },
            Some(Key::Printable(key)) => {
                let key = char::from(key);
                match key {
                    '\x08' => {
                        if self.cursor_pos > 0 {
                            self.value.remove(self.cursor_pos - 1);
                            self.cursor_pos -= 1;
                        }
                    } // backspace
                    key => {
                        self.value.insert(self.cursor_pos, key);
                        self.cursor_pos += 1;
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn save_to_field(&mut self) {
        self.fields[self.idx].1 = self.value.clone();
    }

    fn load_from_field(&mut self) {
        self.value = self.fields[self.idx].1.clone();
        self.cursor_pos = self.value.chars().count();
    }

    fn save_to_config(&self, config: &mut Config) {
        for (key, val) in &self.fields {
            let some_val = match val.clone() {
                val if !val.is_empty() => Some(val),
                _ => None,
            };
            match *key {
                "title" => config.title = some_val,
                "version" => config.version = some_val,
                "machine_id" => config.machine_id = some_val,
                "sort_key" => config.sort_key = some_val,
                "options" => config.options = some_val,
                "devicetree" => config.devicetree = some_val,
                "architecture" => config.architecture = some_val,
                "efi" => config.efi.clone_from(val),
                _ => (),
            }
        }
    }
}
