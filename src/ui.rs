//! The user interface of the bootloader.
//!
//! The overall design of the UI is very heavily inspired off of text-only bootloaders like Microsoft's bootmgr and
//! systemd-boot. The architecture of the UI is built upon ratatui, which means that it is quite extensible and a
//! more complicated UI could be created. However, such a UI would probably be slower than the current UI.
//!
//! The theme of the UI can be changed through the bootloader's config file. There is support for changing the color,
//! and the highlight color.

use alloc::{format, vec, vec::Vec};
use ratatui_core::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{StatefulWidget, Widget},
};
use ratatui_widgets::{block::Block, borders::Borders, list::List, paragraph::Paragraph};

use crate::app::App;

/// App widget implementation.
mod widget;

pub mod boot_list;
pub mod ratatui_backend;
pub mod theme;

impl App {
    /// Renders a `BootList`.
    pub fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let list = List::new(self.boot_list.items.clone())
            .style(self.theme.base)
            .highlight_style(self.theme.highlight)
            .highlight_symbol(" → ");

        StatefulWidget::render(list, area, buf, &mut self.boot_list.state);
    }

    /// Renders the name of the program, as well as the version number.
    pub fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let header_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());
        let header = Paragraph::new(Text::styled(
            format!("bootmgr-rs {}", env!("CARGO_PKG_VERSION")),
            self.theme.base,
        ))
        .alignment(Alignment::Center)
        .block(header_block);

        Widget::render(header, area, buf);
    }

    /// Renders the timeout below the `BootList`.
    pub fn render_timeout(&self, area: Rect, buf: &mut Buffer) {
        let mut text = Line::raw(" ")
            .style(self.theme.base)
            .alignment(Alignment::Center);
        if self.timeout.is_positive() {
            text.push_span(format!("Booting in {} seconds", self.timeout));
        }
        Widget::render(text, area, buf);
    }

    /// Renders the help bar at the bottom of the screen.
    pub fn render_help(&self, area: Rect, buf: &mut Buffer) {
        let mut keys = vec![
            ("↑/W", "Up"),
            ("↓/S", "Down"),
            ("Return", "Start"),
            ("ESC", "Exit"),
            ("+/=", "Toggle Default"),
        ];
        if cfg!(feature = "editor") && self.boot_mgr.boot_config.editor {
            keys.push(("E", "Editor"));
        }
        let spans: Vec<_> = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(format!(" {key} "), self.theme.highlight);
                let desc = Span::styled(format!(" {desc} "), self.theme.base);
                [key, desc]
            })
            .collect();
        Line::from(spans)
            .centered()
            .style(Style::default())
            .render(area, buf);
    }

    /// Renders a status, which is currently used only for indicating setting default.
    pub fn render_status(&self, area: Rect, buf: &mut Buffer) {
        let mut lines = Vec::with_capacity(2);
        if self.set_default {
            let line = Line::raw("Setting default boot option")
                .style(self.theme.base)
                .alignment(Alignment::Center);

            lines.push(line);
        }

        let text = Text::from(lines);
        Widget::render(text, area, buf);
    }
}
