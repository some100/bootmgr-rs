use ratatui_core::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::Widget,
};

use crate::editor::Editor;

impl Widget for &mut Editor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let vertical = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

        self.render_help(vertical[2], buf);
        self.render_editor(vertical[1], buf);
        self.render_title(vertical[0], buf);
    }
}
