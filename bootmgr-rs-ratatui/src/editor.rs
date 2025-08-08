//! The optional basic editor for modifying [`Config`]s.
//!
//! This can be used for editing Linux EFISTUB cmdline options as an example. The options field can be edited to change the
//! necessary parameters.
//!
//! The [`Config`] may also be persistently saved to the filesystem as well. This creates an overlay where the options
//! specified in the overlay will be applied to the [`Config`] if it exists the next time it is booted. Generally, this
//! will be done according to the filename of the [`Config`].

use alloc::borrow::ToOwned;
use ratatui_core::{layout::Position, terminal::Terminal};
use uefi::{
    Event,
    boot::{self, ScopedProtocol},
    proto::console::text::{Input, Key, ScanCode},
};

use bootmgr_rs_core::{
    BootResult,
    config::{Config, editor::ConfigEditor},
};

use crate::{
    app::AppError,
    editor::persist::PersistentConfig,
    ui::{ratatui_backend::UefiBackend, theme::Theme},
};

pub mod persist;

mod ui;
mod widget;

/// The state of the editor.
#[derive(Default, PartialEq, Eq)]
pub enum EditorState {
    /// The [`Editor`] is currently not open.
    #[default]
    Idle,

    /// The [`Editor`] is currently open and is editing a field.
    Editing,

    /// The [`Editor`] is exiting, and is persisting the contents to the filesystem.
    Persisting,

    /// The [`Editor`] is exiting, and is deleting the contents from the filesystem.
    Deleting,
}

/// The basic editor
#[derive(Default)]
pub struct Editor {
    /// The state of the editor
    pub state: EditorState,

    /// The [`ConfigEditor`].
    pub edit: ConfigEditor,

    /// Stores the `wait_for_key` event.
    pub events: Option<[Event; 1]>,

    /// Tracks the current position of the cursor.
    pub cursor_pos: usize,

    /// Stores the collection of persistently saved [`Config`]s.
    pub persist: PersistentConfig,

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
    pub fn new(
        input: &ScopedProtocol<Input>,
        theme: Theme,
        persist: PersistentConfig,
    ) -> Result<Self, AppError> {
        Ok(Self {
            events: Some([input.wait_for_key_event().ok_or(AppError::InputClosed)?]),
            persist,
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
        if let (Some(fg), Some(bg)) = (self.theme.base.fg, self.theme.base.bg) {
            terminal.backend_mut().set_color(fg, bg);
        }

        terminal.clear()?;

        self.edit = ConfigEditor::new(config);

        self.cursor_pos = self.edit.current_field().chars().count();

        while self.state == EditorState::Editing {
            self.draw(terminal)?;

            let cursor_pos = u16::try_from(self.cursor_pos).unwrap_or(u16::MAX);
            terminal.set_cursor_position(Position::new(cursor_pos, 3))?; // top bar is ALWAYS 3 length

            self.wait_for_events();
            self.handle_key(input)?;
        }

        self.edit.build(config);

        if self.state == EditorState::Editing {
            if !self.persist.contains(config) {
                self.persist.add_config_to_persist(config);
            }
            let _ = self.persist.save_to_fs();
        } else if self.state == EditorState::Deleting {
            self.persist.remove_config_from_persist(config);
            let _ = self.persist.save_to_fs();
        }

        terminal.hide_cursor()?;

        Ok(())
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
    /// If the key is F1, then the values will be saved to the filesystem persistently and the editor exits.
    fn handle_special_key(&mut self, key: ScanCode) {
        match key {
            ScanCode::ESCAPE => {
                self.state = EditorState::Idle;
            }
            ScanCode::FUNCTION_1 => {
                self.state = EditorState::Persisting;
            }
            ScanCode::FUNCTION_2 => {
                self.state = EditorState::Deleting;
            }
            ScanCode::UP => {
                self.edit.prev_field();
                self.cursor_pos = self.edit.chars();
            }
            ScanCode::DOWN => {
                self.edit.next_field();
                self.cursor_pos = self.edit.chars();
            }
            ScanCode::LEFT => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            ScanCode::RIGHT => {
                self.cursor_pos = (self.cursor_pos + 1).min(self.edit.chars());
            }
            _ => (),
        }
    }

    /// Handle a printable key.
    ///
    /// If the key is a backspace, then it will remove the current value and push the cursor position back by one.
    /// If the key is anything else, then that key will be inserted into the current value.
    fn handle_printable_key(&mut self, key: char) {
        let mut value = self.edit.current_field().to_owned();
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
        self.edit.update_selected(&value);
    }
}
