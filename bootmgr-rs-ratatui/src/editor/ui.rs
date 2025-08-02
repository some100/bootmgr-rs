//! The user interface of the editor.
//!
//! This is a highly simplistic user interface that only features a title, an editor area, and a help bar at the bottom.

use alloc::{format, vec::Vec};
use bootmgr_rs_core::BootResult;
use ratatui_core::{
    buffer::Buffer, layout::{Alignment, Rect}, style::Style, terminal::Terminal, text::{Line, Span, Text}, widgets::Widget
};
use ratatui_widgets::{block::Block, borders::Borders, paragraph::Paragraph};

use crate::{editor::Editor, ui::ratatui_backend::UefiBackend};

impl Editor {
    pub fn draw(&mut self, terminal: &mut Terminal<UefiBackend>) -> BootResult<()> {
        terminal.draw(|f| f.render_widget(self, f.area()))?;
        terminal.show_cursor()?;
        Ok(())
    }

    /// Displays the currently edited field of the `Config`.
    pub fn render_title(&self, area: Rect, buf: &mut Buffer) {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());
        let title = Paragraph::new(Text::styled(
            format!("Currently editing {}", self.fields[self.idx].0),
            self.theme.base,
        ))
        .block(title_block);

        Widget::render(title, area, buf);
    }

    /// Displays the content of the current field.
    pub fn render_editor(&self, area: Rect, buf: &mut Buffer) {
        let text = Line::raw(&self.fields[self.idx].1)
            .style(self.theme.base)
            .alignment(Alignment::Left);

        Widget::render(text, area, buf);
    }

    /// Displays the help bar on the bottom of the screen.
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
                let key = Span::styled(format!(" {key} "), self.theme.highlight);
                let desc = Span::styled(format!(" {desc} "), self.theme.base);
                [key, desc]
            })
            .collect();
        let line = Line::from(spans).centered().style(Style::default());

        Widget::render(line, area, buf);
    }
}
