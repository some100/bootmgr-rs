//! App widget implementation.

use ratatui_core::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::Widget,
};

use crate::app::App;

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // two areas on the bottom for the help and default tip
        let vertical = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

        // leave a room of length 10 for the list
        let middle = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(10),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(vertical[0]);

        // have the list only take up 50% of the screen width wise
        let horizontal = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(20),
        ])
        .split(middle[1]);

        self.render_help(vertical[2], buf);
        self.render_status(vertical[1], buf);
        self.render_timeout(middle[3], buf);
        self.render_list(horizontal[1], buf);
    }
}
