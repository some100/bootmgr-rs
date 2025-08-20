// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! The user interface of the bootloader.
//!
//! The overall design of the UI is very heavily inspired off of text-only bootloaders like Microsoft's bootmgr and
//! systemd-boot. The architecture of the UI is built upon ratatui, which means that it is quite extensible and a
//! more complicated UI could be created. However, such a UI would probably be slower than the current UI.
//!
//! The theme of the UI can be changed through the bootloader's config file. There is support for changing the color,
//! and the highlight color.

use alloc::format;
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
use tinyvec::ArrayVec;

use crate::{MainError, app::App, ui::ratatui_backend::UefiBackend};

mod widget;

pub mod boot_list;
pub mod ratatui_backend;
pub mod theme;

impl App {
    /// Draw a frame to the screen.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the widgets could not be drawn to the screen.
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
            concat!("bootmgr-rs ", env!("CARGO_PKG_VERSION")),
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
        const KEYS: [(&str, &str); 5] = [
            (" ↑/W ", " Up "),
            (" ↓/S ", " Down "),
            (" Return ", " Start "),
            (" ESC ", " Exit "),
            (" +/= ", " Toggle Default "),
        ];

        let mut spans: ArrayVec<[_; 12]> = ArrayVec::new();

        for (key, desc) in &KEYS {
            spans.push(Span::styled(*key, self.theme.highlight));
            spans.push(Span::styled(*desc, self.theme.base));
        }

        #[cfg(feature = "editor")]
        if self.boot_mgr.boot_config.editor {
            spans.push(Span::styled(" E ", self.theme.highlight));
            spans.push(Span::styled(" Editor ", self.theme.base));
        }

        Line::default()
            .spans(spans)
            .centered()
            .style(Style::default())
            .render(area, buf);
    }

    /// Renders a status, which is currently used only for indicating setting default.
    pub fn render_status(&self, area: Rect, buf: &mut Buffer) {
        let mut lines: ArrayVec<[_; 2]> = ArrayVec::new();
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
