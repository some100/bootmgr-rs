//! The user interface of the bootloader.
//!
//! The overall design of the UI is very heavily inspired off of text-only bootloaders like Microsoft's bootmgr and
//! systemd-boot. The architecture of the UI is built upon ratatui, which means that it is quite extensible and a
//! more complicated UI could be created. However, such a UI would probably be slower than the current UI.
//!
//! The theme of the UI can be changed through the bootloader's config file. There is support for changing the color,
//! and the highlight color.

use alloc::{format, vec::Vec};
use ratatui_core::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    terminal::Terminal,
    text::{Line, Span, Text},
    widgets::{StatefulWidget, Widget},
};
use ratatui_widgets::{
    block::Block,
    borders::Borders,
    list::{List, ListItem},
    paragraph::Paragraph,
};
use smallvec::{SmallVec, smallvec};

use crate::{MainError, app::App, ui::ratatui_backend::UefiBackend};

/// App widget implementation.
mod widget;

pub mod boot_list;
pub mod ratatui_backend;
pub mod theme;

impl App {
    /// Draw a frame to the screen.
    pub fn draw(&mut self, terminal: &mut Terminal<UefiBackend>) -> Result<(), MainError> {
        terminal.draw(|f| f.render_widget(self, f.area()))?;
        Ok(())
    }
    /// Renders a `BootList`.
    pub fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let list = List::new(self.boot_list.items.iter().map(|x| ListItem::new(&**x)))
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
        let mut keys: SmallVec<[_; 6]> = smallvec![
            ("↑/W", "Up"),
            ("↓/S", "Down"),
            ("Return", "Start"),
            ("ESC", "Exit"),
            ("+/=", "Toggle Default"),
        ];

        #[cfg(feature = "editor")]
        if self.boot_mgr.boot_config.editor {
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
        let mut lines: SmallVec<[_; 2]> = SmallVec::new();
        if self.set_default {
            let line = Line::raw("Setting default boot option")
                .style(self.theme.base)
                .alignment(Alignment::Center);

            lines.push(line);
        }

        let text = lines.into_iter().collect::<Text>();
        Widget::render(text, area, buf);
    }
}
