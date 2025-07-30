//! The optional basic editor for modifying [`Config`]s.
//!
//! The modifications made by the editor are not persistent. They remain only in memory. Any long term modifications
//! should be done in an actual operating system environment. It's still useful for editing boot options if the need
//! ever arises.

use alloc::{string::String, vec::Vec};
use ratatui_core::{layout::Position, terminal::Terminal};
use uefi::{
    Event,
    boot::{self, ScopedProtocol},
    proto::console::text::{Input, Key, ScanCode},
};

use crate::{
    BootResult,
    app::AppError,
    config::{Config, builder::ConfigBuilder},
    system::helper::truncate_usize_to_u16,
    ui::{ratatui_backend::UefiBackend, theme::Theme},
};

mod ui;

/// Editor widget implementation.
mod widget;

/// The basic editor
#[derive(Default)]
pub struct Editor {
    /// Checks if the editor is currently editing.
    pub editing: bool,

    /// Stores the `wait_for_key` event.
    pub events: Option<[Event; 1]>,

    /// Tracks the current position of the cursor.
    pub cursor_pos: usize,

    /// Stores the fields that are in the [`Config`].
    pub fields: Vec<(&'static str, String)>,

    /// Stores which field is currently being edited.
    pub idx: usize,

    /// Stores the value of the field.
    pub value: String,

    /// Stores the [`Theme`] of the UI.
    pub theme: Theme,
}

impl Editor {
    /// Creates a new [`Editor`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the [`Input`] protocol was already closed before the [`Editor`]
    /// was initialized.
    pub fn new(input: &ScopedProtocol<Input>, theme: Theme) -> BootResult<Self> {
        Ok(Self {
            events: Some([input.wait_for_key_event().ok_or(AppError::InputClosed)?]),
            theme,
            ..Self::default()
        })
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
    ) -> BootResult<()> {
        if let Some(fg) = self.theme.base.fg
            && let Some(bg) = self.theme.base.bg
        {
            terminal.backend_mut().set_color(fg, bg);
        }

        terminal.clear()?;

        self.init_state(config);

        loop {
            terminal.draw(|f| f.render_widget(&mut *self, f.area()))?;
            terminal.show_cursor()?;

            let cursor_pos = truncate_usize_to_u16(self.cursor_pos);
            terminal.set_cursor_position(Position::new(cursor_pos, 3))?; // top bar is ALWAYS 3 length

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

    /// Reads the [`Config`] file into the field and initializes the state
    fn init_state(&mut self, config: &Config) {
        self.fields = config
            .get_str_fields()
            .into_iter()
            .map(|(k, v)| (k, v.cloned().unwrap_or_default()))
            .collect();
        self.value = self.fields[0].1.clone();
        self.cursor_pos = self.value.chars().count();
        self.idx = 0;
    }

    /// Wait for the key event.
    fn wait_for_events(&mut self) {
        let Some(events) = &mut self.events else {
            return;
        };

        let _ = boot::wait_for_event(events);
    }

    /// Handle a key that was pressed.
    ///
    /// # Errors
    ///
    /// May return an `Error` if there was some sort of device error with the [`Input`].
    fn handle_key(&mut self, input: &mut ScopedProtocol<Input>) -> BootResult<()> {
        match input.read_key()? {
            Some(Key::Special(key)) => self.handle_special_key(key),
            Some(Key::Printable(key)) => self.handle_printable_key(key.into()),
            _ => (),
        }
        Ok(())
    }

    /// Handle a special key.
    ///
    /// If the key is an escape, then the values are saved into the config field and the editor exits.
    /// If the key is up or down, then the current field will be saved and a new field will be loaded.
    /// If the key is left or right, then the cursor position is moved.
    fn handle_special_key(&mut self, key: ScanCode) {
        match key {
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
        }
    }

    /// Handle a printable key.
    ///
    /// If the key is a backspace, then it will remove the current value and push the cursor position back by one.
    /// If the key is anything else, then that key will be inserted into the current value.
    fn handle_printable_key(&mut self, key: char) {
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

    /// Save the current value into the config field.
    fn save_to_field(&mut self) {
        self.fields[self.idx].1 = self.value.clone();
    }

    /// Load a new value and cursor position from a config field.
    fn load_from_field(&mut self) {
        self.value = self.fields[self.idx].1.clone();
        self.cursor_pos = self.value.chars().count();
    }

    /// Parse the fields of the editor back into the [`Config`].
    ///
    /// This only makes in memory changes to the [`Config`], because it is impossible at this stage to determine
    /// the origin of the [`Config`]. If the [`Config`] originated from a Windows BCD, or a UKI executable, it would
    /// not be possible to change the configuration options permanently (without significantly more complicated logic).
    fn save_to_config(&self, config: &mut Config) {
        let mut config = ConfigBuilder::from(config.clone());
        for (key, val) in &self.fields {
            config = match *key {
                "title" => config.title(val),
                "version" => config.version(val),
                "machine_id" => config.machine_id(val),
                "sort_key" => config.sort_key(val),
                "options" => config.options(val),
                "devicetree" => config.devicetree(val),
                "architecture" => config.architecture(val),
                "efi" => config.efi(val),
                _ => config,
            }
        }
    }
}
