//! The optional basic editor for modifying [`Config`]s.
//!
//! This can be used for editing Linux EFISTUB cmdline options as an example. The options field can be edited to change the
//! necessary parameters.
//!
//! Due to the diversity of [`Config`]'s that may be supplied to the boot manager, as well as the fact that some of
//! the [`Config`]'s sources may not be edited or mutable, there are no plans to add persistent boot editing.

use alloc::string::String;
use ratatui_core::{layout::Position, terminal::Terminal};
use smallvec::SmallVec;
use uefi::{
    Event,
    boot::{self, ScopedProtocol},
    proto::console::text::{Input, Key, ScanCode},
};

use bootmgr_rs_core::{
    BootResult,
    config::{Config, builder::ConfigBuilder},
    system::helper::truncate_usize_to_u16,
};

use crate::{
    app::AppError,
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
    pub fields: SmallVec<[(&'static str, String); 8]>,

    /// Stores which field is currently being edited.
    pub idx: usize,

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
    pub fn new(input: &ScopedProtocol<Input>, theme: Theme) -> Result<Self, AppError> {
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
            self.draw(terminal)?;

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
        self.fields.extend(
            config
                .get_str_fields()
                .iter()
                .map(|(k, v)| (*k, v.cloned().unwrap_or_default())),
        );

        self.cursor_pos = self.fields[0].1.chars().count();
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
        let value = &self.fields[self.idx].1;
        match key {
            ScanCode::ESCAPE => {
                self.editing = false;
            }
            ScanCode::UP => {
                if self.idx > 0 {
                    self.idx -= 1;
                }
                self.cursor_pos = value.chars().count();
            }
            ScanCode::DOWN => {
                if self.idx + 1 < self.fields.len() {
                    self.idx += 1;
                }
                self.cursor_pos = self.fields[self.idx].1.chars().count();
            }
            ScanCode::LEFT => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            ScanCode::RIGHT => {
                self.cursor_pos = (self.cursor_pos + 1).min(value.len());
            }
            _ => (),
        }
    }

    /// Handle a printable key.
    ///
    /// If the key is a backspace, then it will remove the current value and push the cursor position back by one.
    /// If the key is anything else, then that key will be inserted into the current value.
    fn handle_printable_key(&mut self, key: char) {
        let value = &mut self.fields[self.idx].1;
        match key {
            '\x08' => {
                if self.cursor_pos > 0 {
                    value.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
            } // backspace
            key => {
                value.insert(self.cursor_pos, key);
                self.cursor_pos += 1;
            }
        }
    }

    /// Parse the fields of the editor back into the [`Config`].
    ///
    /// This only makes in memory changes to the [`Config`], because it is impossible at this stage to determine
    /// the origin of the [`Config`]. If the [`Config`] originated from a Windows BCD, or a UKI executable, it would
    /// not be possible to change the configuration options permanently (without significantly more complicated logic).
    fn save_to_config(&self, config: &mut Config) {
        let mut builder = ConfigBuilder::from(&*config);
        for (key, val) in &self.fields {
            builder = match *key {
                "title" => builder.title(val),
                "version" => builder.version(val),
                "machine_id" => builder.machine_id(val),
                "sort_key" => builder.sort_key(val),
                "options" => builder.options(val),
                "devicetree" => builder.devicetree(val),
                "architecture" => builder.architecture(val),
                "efi" => builder.efi(val),
                _ => builder,
            };
        }
        *config = builder.build();
    }
}
