//! Provides [`Theme`], which exposes the color scheme for the UI.

use ratatui_core::style::Style;

use crate::boot::config::BootConfig;

/// The color scheme of the UI.
#[derive(Clone, Copy, Default)]
pub struct Theme {
    /// The color scheme for everything except highlighted items.
    pub base: Style,

    /// The color scheme for highlighted items.
    pub highlight: Style,
}

impl Theme {
    /// Create a new [`Theme`] from a [`BootConfig`].
    #[must_use = "Has no effect if the result is unused"]
    pub fn new(config: &BootConfig) -> Self {
        Self {
            base: Style::new().fg(config.fg).bg(config.bg),
            highlight: Style::new().fg(config.highlight_fg).bg(config.highlight_bg),
        }
    }
}
