//! Rendering: laying out the panel and handing each pane its rectangle.
//!
//! This sits alongside [`app`](crate::app) and reads the `pub(crate)` state it
//! exposes. It only ever *reads* that state — all mutation lives in the panes
//! and the `app` loop — which is why it's worth keeping separate. The drawing of
//! each pane lives with that pane; this just decides where each one goes.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
};

use crate::app::{App, Focus};

impl App {
    /// Carve the screen into panes and hand each its rectangle to draw into.
    pub(crate) fn draw(&mut self, frame: &mut Frame) {
        // Inset the panel from the screen edge so it reads as a floating window
        // rather than filling the terminal.
        let area = frame.area();
        let panel = Rect {
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
        };

        // Split the panel into the task list on the left and a side column on the
        // right, which itself stacks a notes/refinement pane over a subtasks pane.
        // When the side column is hidden (toggled with `n`), the list takes the
        // whole panel.
        let (list_area, notes_area, subtasks_area) = if self.show_notes {
            let [list_area, right_area] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .areas(panel);
            let [notes_area, subtasks_area] = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(right_area);
            (list_area, Some(notes_area), Some(subtasks_area))
        } else {
            (panel, None, None)
        };

        // The list pane owns its own accent (it shifts to yellow while editing a
        // title); the side panes use a steady accent and only light their border
        // when focused.
        self.tasks_pane
            .render(&self.tasks, list_area, frame, self.focus == Focus::List);

        // The task whose notes + subtasks the side panes reflect. `None` when the
        // list is empty, in which case the panes show their empty state. The
        // panes know how to draw; we just hand them the data and the rectangle.
        let accent = Color::Cyan;
        let selected = self.tasks_pane.selected().map(|i| &self.tasks[i]);
        if let Some(notes_area) = notes_area {
            let notes = selected.map(|t| &t.notes);
            let border = if self.focus == Focus::Notes {
                accent
            } else {
                Color::Black
            };
            self.notes.render(notes, notes_area, frame, border, accent);
        }
        if let Some(subtasks_area) = subtasks_area {
            let subtasks = selected.map(|t| t.subtasks.as_slice()).unwrap_or(&[]);
            let border = if self.focus == Focus::Subtasks {
                accent
            } else {
                Color::Black
            };
            self.subtasks
                .render(subtasks, subtasks_area, frame, border, accent);
        }
    }
}
