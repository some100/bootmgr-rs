//! Stubs for features that are disabled

#[cfg(feature = "editor")]
pub mod editor {
    pub use crate::editor::*;
}

#[cfg(not(feature = "editor"))]
pub mod editor {
    use ratatui_core::terminal::Terminal;
    use uefi::{boot::ScopedProtocol, proto::console::text::Input};

    use crate::{config::Config, error::BootError, ui::ratatui_backend::UefiBackend};

    #[derive(Default)]
    pub struct Editor {
        pub editing: bool,
    }

    impl Editor {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn run(
            &mut self,
            config: &mut Config,
            input: &mut ScopedProtocol<Input>,
            terminal: &mut Terminal<UefiBackend>,
        ) -> Result<(), BootError> {
            let _ = config;
            let _ = input;
            let _ = terminal;
            self.editing = false;
            Ok(())
        }
    }
}
