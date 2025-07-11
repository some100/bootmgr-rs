//! The user interface of the bootloader

use alloc::{format, vec, vec::Vec};
use ratatui_core::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{StatefulWidget, Widget},
};
use ratatui_widgets::list::List;

use crate::app::App;

mod widget;

pub mod ratatui_backend;

impl App {
    /// Renders a `BootList`.
    pub fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let list = List::new(self.boot_list.items.clone())
            .style(
                Style::new()
                    .fg(self.boot_mgr.boot_config.fg)
                    .bg(self.boot_mgr.boot_config.bg),
            )
            .highlight_style(
                Style::new()
                    .fg(self.boot_mgr.boot_config.highlight_fg)
                    .bg(self.boot_mgr.boot_config.highlight_bg),
            )
            .highlight_symbol(" → ");

        StatefulWidget::render(list, area, buf, &mut self.boot_list.state);
    }

    /// Renders the timeout below the `BootList`.
    pub fn render_timeout(&self, area: Rect, buf: &mut Buffer) {
        let mut text = Line::raw(" ")
            .style(
                Style::new()
                    .fg(self.boot_mgr.boot_config.fg)
                    .bg(self.boot_mgr.boot_config.bg),
            )
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
                let key = Span::styled(
                    format!(" {key} "),
                    Style::new()
                        .fg(self.boot_mgr.boot_config.highlight_fg)
                        .bg(self.boot_mgr.boot_config.highlight_bg),
                );
                let desc = Span::styled(
                    format!(" {desc} "),
                    Style::new()
                        .fg(self.boot_mgr.boot_config.fg)
                        .bg(self.boot_mgr.boot_config.bg),
                );
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
                .style(
                    Style::new()
                        .fg(self.boot_mgr.boot_config.fg)
                        .bg(self.boot_mgr.boot_config.bg),
                )
                .alignment(Alignment::Center);

            lines.push(line);
        }

        let text = Text::from(lines);
        Widget::render(text, area, buf);
    }
}
