use alloc::{format, vec::Vec};
use ratatui_core::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};
use ratatui_widgets::list::List;

use crate::app::App;

mod widget;

pub mod ratatui_backend;

impl App {
    pub fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let list = List::new(self.boot_options.items.clone())
            .highlight_style(Style::new().fg(Color::Black).bg(Color::Gray))
            .highlight_symbol(" → ");

        StatefulWidget::render(list, area, buf, &mut self.boot_options.state);
    }

    pub fn render_timeout(&self, area: Rect, buf: &mut Buffer) {
        if self.timeout.is_positive() {
            let text = Line::raw(format!("Booting in {} seconds", self.timeout))
                .style(Style::new().fg(Color::White).bg(Color::Black))
                .alignment(Alignment::Center);

            Widget::render(text, area, buf);
        }
    }

    pub fn render_help(area: Rect, buf: &mut Buffer) {
        let keys = [
            ("↑/W", "Up"),
            ("↓/S", "Down"),
            ("Return", "Start"),
            ("ESC", "Exit"),
            ("+/=", "Toggle Default"),
        ];
        let spans: Vec<_> = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(
                    format!(" {key} "),
                    Style::new().fg(Color::Black).bg(Color::Gray),
                );
                let desc = Span::styled(
                    format!(" {desc} "),
                    Style::new().fg(Color::Gray).bg(Color::Black),
                );
                [key, desc]
            })
            .collect();
        Line::from(spans)
            .centered()
            .style(Style::default())
            .render(area, buf);
    }

    pub fn render_default(&self, area: Rect, buf: &mut Buffer) {
        if self.set_default {
            let text = Line::raw("Setting default boot option")
                .style(Style::new().fg(Color::White).bg(Color::Black))
                .alignment(Alignment::Center);

            Widget::render(text, area, buf);
        }
    }
}
