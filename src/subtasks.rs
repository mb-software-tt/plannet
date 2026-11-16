//! The subtasks slice: the child checklist under a task.
//!
//! Like the notes slice, this owns three things: the data type (`Subtask`), the
//! pane's interaction state and input handling (`SubtasksPane`), and how both
//! are drawn. The shell just routes keys to the focused pane and hands it a
//! rectangle.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Padding, Paragraph},
};
use serde::{Deserialize, Serialize};

use crate::app::Outcome;

/// A single child item under a task. Same shape as a top-level task today, but
/// kept as its own type: subtasks have no life of their own (they belong to a
/// task), and giving them a distinct type lets the two evolve independently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub title: String,
    pub done: bool,
}

/// The subtasks pane's interaction state — the component the shell routes keys
/// to when the subtasks pane is focused.
///
/// `state` tracks which subtask is highlighted; `adding` is `Some` while the
/// user is typing a new subtask (single-line, so a plain `String` buffer is
/// enough — no text area needed).
#[derive(Default)]
pub struct SubtasksPane {
    state: ListState,
    adding: Option<String>,
}

impl SubtasksPane {
    /// Handle a key while the subtasks pane has focus. POC scope: navigate
    /// (`j`/`k`), toggle done (`space`), add (`a`). While adding, every key is
    /// ours; otherwise unknown keys pass back to the shell.
    pub fn handle_key(&mut self, key: KeyEvent, subtasks: Option<&mut Vec<Subtask>>) -> Outcome {
        // Without a selected task there's nothing to attach subtasks to.
        let Some(subtasks) = subtasks else {
            return Outcome::Pass;
        };

        if let Some(buffer) = &mut self.adding {
            match key.code {
                KeyCode::Enter => {
                    let title = buffer.trim().to_string();
                    if !title.is_empty() {
                        subtasks.push(Subtask { title, done: false });
                        self.state.select(Some(subtasks.len() - 1));
                    }
                    self.adding = None;
                }
                KeyCode::Esc => self.adding = None, // discard the half-typed subtask
                KeyCode::Backspace => {
                    buffer.pop();
                }
                KeyCode::Char(c) => buffer.push(c),
                _ => {}
            }
            return Outcome::Consumed;
        }

        match key.code {
            KeyCode::Char('a') => {
                self.adding = Some(String::new());
                Outcome::Consumed
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next();
                Outcome::Consumed
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_previous();
                Outcome::Consumed
            }
            KeyCode::Char(' ') => {
                if let Some(sub) = self.state.selected().and_then(|i| subtasks.get_mut(i)) {
                    sub.done = !sub.done;
                }
                Outcome::Consumed
            }
            _ => Outcome::Pass,
        }
    }

    /// Paint the pane: the checklist with the selected row highlighted, plus a
    /// live input row while adding. Falls back to a dimmed prompt when empty.
    pub fn render(
        &mut self,
        subtasks: &[Subtask],
        area: Rect,
        frame: &mut Frame,
        border: Color,
        accent: Color,
    ) {
        // Keep the selection valid as the count changes (e.g. switching tasks).
        match self.state.selected() {
            _ if subtasks.is_empty() => self.state.select(None),
            None => self.state.select(Some(0)),
            Some(i) if i >= subtasks.len() => self.state.select(Some(subtasks.len() - 1)),
            _ => {}
        }

        let block = pane_block(border, accent);

        if subtasks.is_empty() && self.adding.is_none() {
            let widget = Paragraph::new("No subtasks yet. Press 'a' to add one.")
                .style(Style::new().add_modifier(Modifier::DIM))
                .block(block);
            frame.render_widget(widget, area);
            return;
        }

        let mut items: Vec<ListItem> = subtasks.iter().map(subtask_item).collect();
        if let Some(buffer) = &self.adding {
            // A block cursor (█) marks the row being typed.
            items.push(ListItem::new(format!("☐ {buffer}\u{2588}")));
        }

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().fg(accent).add_modifier(Modifier::BOLD))
            .highlight_symbol("❯ ");
        frame.render_stateful_widget(list, area, &mut self.state);
    }
}

/// One checklist row. Done items recede: green check, title dimmed + struck out.
fn subtask_item(sub: &Subtask) -> ListItem<'static> {
    let checkbox = if sub.done { "✓" } else { "☐" };
    if sub.done {
        ListItem::new(Line::from(vec![
            Span::styled(format!("{checkbox} "), Style::new().fg(Color::Green)),
            Span::styled(
                sub.title.clone(),
                Style::new().add_modifier(Modifier::DIM | Modifier::CROSSED_OUT),
            ),
        ]))
    } else {
        ListItem::new(format!("{checkbox} {}", sub.title))
    }
}

/// The bordered box for the subtasks pane.
fn pane_block(border: Color, accent: Color) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border))
        .title(Span::styled(
            " subtasks ",
            Style::new().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::horizontal(1))
}
