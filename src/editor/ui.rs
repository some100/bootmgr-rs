use alloc::{format, vec::Vec};
use ratatui_core::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::Widget,
};
use ratatui_widgets::{block::Block, borders::Borders, paragraph::Paragraph};

use crate::editor::Editor;

impl Editor {
    pub fn render_title(&self, area: Rect, buf: &mut Buffer) {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());
        let title = Paragraph::new(Text::styled(
            format!("Currently editing {}", self.fields[self.idx].0),
            Style::new().fg(self.fg).bg(self.bg),
        ))
        .block(title_block);

        Widget::render(title, area, buf);
    }

    pub fn render_editor(&self, area: Rect, buf: &mut Buffer) {
        let text = Line::raw(&self.value)
            .style(Style::new().fg(self.fg).bg(self.bg))
            .alignment(Alignment::Left);

        Widget::render(text, area, buf);
    }

    pub fn render_help(&self, area: Rect, buf: &mut Buffer) {
        let keys = [
            ("↑/↓", "Previous/Next Field"),
            ("←/→", "Move Cursor"),
            ("Any Key", "Edit"),
            ("ESC", "Exit"),
        ];
        let spans: Vec<_> = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(format!(" {key} "), Style::new().fg(self.bg).bg(self.fg));
                let desc = Span::styled(format!(" {desc} "), Style::new().fg(self.fg).bg(self.bg));
                [key, desc]
            })
            .collect();
        let line = Line::from(spans).centered().style(Style::default());

        Widget::render(line, area, buf);
    }
}
