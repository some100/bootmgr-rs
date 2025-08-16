// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Stubs for features that are disabled

/// The editor feature.
#[cfg(feature = "editor")]
pub mod editor {
    pub use crate::editor::*;
}

/// The editor feature.
#[cfg(not(feature = "editor"))]
pub mod editor {
    use ratatui_core::terminal::Terminal;
    use uefi::{boot::ScopedProtocol, proto::console::text::Input};

    use bootmgr_rs_core::{BootResult, config::Config};

    use crate::ui::{ratatui_backend::UefiBackend, theme::Theme};

    pub struct PersistentConfig;

    impl PersistentConfig {
        pub fn new() -> BootResult<Self> {
            Ok(Self)
        }

        pub fn swap_config_in_persist(&self, _config: &mut Config) {}
    }

    /// A disabled editor. Has only one field, which does nothing.
    #[derive(Default)]
    pub struct Editor {
        /// A field that tracks if the editor is editing. Because the editor is disabled, it does nothing.
        pub editing: bool,
    }

    impl Editor {
        /// # Errors
        ///
        /// None
        #[must_use = "Has no effect if the result is unused"]
        pub fn new(
            _input: &ScopedProtocol<Input>,
            _theme: Theme,
            _persist: PersistentConfig,
        ) -> BootResult<Self> {
            Ok(Self::default())
        }

        /// # Errors
        ///
        /// None
        pub fn run(
            &mut self,
            _config: &mut Config,
            _input: &mut ScopedProtocol<Input>,
            _terminal: &mut Terminal<UefiBackend>,
        ) -> BootResult<()> {
            self.editing = false;
            Ok(())
        }
    }
}
