//! The task slice: the root the whole app is organized around.
//!
//! A `Task` *composes* the other domain slices — its notes and its subtasks —
//! rather than spelling out their internals. Each of those types lives in its
//! own slice ([`notes`](crate::notes), [`subtasks`](crate::subtasks)) and owns
//! its own shape; `Task` just holds them together as the unit that persists.
//!
//! Like the other slices, this also owns the pane that drives the task list:
//! [`TaskListPane`] holds the selection + in-progress title edit and knows how
//! to draw the list. The shell just routes keys to it and hands it a rectangle.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Padding, Paragraph},
};
use serde::{Deserialize, Serialize};

use crate::app::Outcome;
use crate::notes::Notes;
use crate::subtasks::Subtask;

/// A single to-do item, plus the refinement notes and child subtasks attached
/// to it.
///
/// The fields are `pub` so other modules (like `app`) can read them; new tasks
/// are built through [`Task::new`] rather than the raw literal, so callers don't
/// have to know what an empty task's notes and subtasks look like.
/// `#[derive(Debug)]` lets us print one with `{:?}` for debugging.
/// `Serialize`/`Deserialize` let serde convert a `Task` to and from TOML so the
/// storage layer can persist it to `.plannet/list.toml`.
///
/// `notes` and `subtasks` are `#[serde(default)]`, so a `list.toml` written
/// before these fields existed still loads cleanly — old tasks come back with
/// empty notes and no subtasks instead of failing to parse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub title: String,
    pub done: bool,
    #[serde(default)]
    pub notes: Notes,
    #[serde(default)]
    pub subtasks: Vec<Subtask>,
}

impl Task {
    /// A fresh task with the given title, not yet done and with no notes or
    /// subtasks attached. The one place that knows how to assemble an empty
    /// task, so adding a field here doesn't ripple out to every caller.
    pub fn new(title: String) -> Self {
        Self {
            title,
            done: false,
            notes: Notes::default(),
            subtasks: Vec::new(),
        }
    }
}

/// The task list pane's interaction state — the component the shell routes keys
/// to when the list has focus.
///
/// `state` tracks which task is highlighted; the shell reads it (via
/// [`selected`](Self::selected)) to know which task the notes/subtasks panes
/// should reflect. `editing` is `Some` while the user is typing a title — both
/// for `a` (a freshly added blank task) and `e` (an existing one). The buffer
/// lives here, so a half-typed title never touches the `Task` until it commits.
#[derive(Default)]
pub struct TaskListPane {
    state: ListState,
    editing: Option<String>,
}

impl TaskListPane {
    /// The highlighted task's index, if any. `None` when the list is empty.
    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Handle a key while the list has focus. While editing a title every key is
    /// ours (so globals like `q`/`Tab` never fire mid-edit); otherwise we own
    /// navigation + the task verbs and pass anything else back to the shell.
    pub fn handle_key(&mut self, key: KeyEvent, tasks: &mut Vec<Task>) -> Outcome {
        // Editing a title: the buffer eats every key. `Enter` commits, `Esc`
        // discards. We `take()` the buffer up front so there's no live borrow of
        // `self.editing` while we call our own `&mut self` helpers below.
        if self.editing.is_some() {
            match key.code {
                KeyCode::Enter => {
                    let title = self.editing.take().unwrap().trim().to_string();
                    // An empty title means "nothing here" — drop the row rather
                    // than keep a blank one (this cleans up an `a` left unfilled).
                    if title.is_empty() {
                        self.remove_selected(tasks);
                    } else if let Some(i) = self.state.selected() {
                        tasks[i].title = title;
                    }
                }
                KeyCode::Esc => {
                    self.editing = None;
                    // Cancelling a brand-new (still-blank) task removes it rather
                    // than leaving an empty row behind.
                    if let Some(i) = self.state.selected()
                        && tasks[i].title.is_empty()
                    {
                        self.remove_selected(tasks);
                    }
                }
                KeyCode::Backspace => {
                    self.editing.as_mut().unwrap().pop();
                }
                KeyCode::Char(c) => self.editing.as_mut().unwrap().push(c),
                _ => {}
            }
            return Outcome::Consumed;
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.select_next();
                Outcome::Consumed
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.select_previous();
                Outcome::Consumed
            }
            KeyCode::Char(' ') => {
                if let Some(task) = self.state.selected().and_then(|i| tasks.get_mut(i)) {
                    task.done = !task.done; // `!` flips a bool
                }
                Outcome::Consumed
            }
            KeyCode::Char('a') => {
                // "Add" is just "make an empty task and edit it": append a blank
                // task, select it, and start typing its title.
                tasks.push(Task::new(String::new()));
                self.state.select(Some(tasks.len() - 1));
                self.editing = Some(String::new());
                Outcome::Consumed
            }
            KeyCode::Char('e') => {
                // Pre-fill the buffer with the current title so the user edits the
                // existing text rather than starting from scratch.
                if let Some(i) = self.state.selected() {
                    self.editing = Some(tasks[i].title.clone());
                    Outcome::Consumed
                } else {
                    Outcome::Pass // nothing selected, let the shell have the key
                }
            }
            _ => Outcome::Pass,
        }
    }

    /// Remove the highlighted task, then fix up the selection so it still points
    /// at a valid row (or `None` if the list is now empty).
    fn remove_selected(&mut self, tasks: &mut Vec<Task>) {
        if let Some(i) = self.state.selected() {
            tasks.remove(i); // shifts later elements down by one
            if tasks.is_empty() {
                self.state.select(None);
            } else if i >= tasks.len() {
                // we removed the last row; move selection up to the new last
                self.state.select(Some(tasks.len() - 1));
            }
        }
    }

    /// Paint the list: the app title + key hints on the border, the tasks with
    /// the selected row highlighted, and a live in-place cursor on the row being
    /// edited. Falls back to a centered prompt when there are no tasks.
    pub fn render(&mut self, tasks: &[Task], area: Rect, frame: &mut Frame, focused: bool) {
        // Keep the selection valid as the count changes (e.g. a task is removed).
        match self.state.selected() {
            _ if tasks.is_empty() => self.state.select(None),
            None => self.state.select(Some(0)),
            Some(i) if i >= tasks.len() => self.state.select(Some(tasks.len() - 1)),
            _ => {}
        }

        // The accent shifts to yellow while editing so the mode is obvious at a
        // glance; the border only picks it up when this pane has focus.
        let accent = if self.editing.is_some() {
            Color::Yellow
        } else {
            Color::Cyan
        };
        let border = if focused { accent } else { Color::Black };

        let done = tasks.iter().filter(|t| t.done).count();
        let title = format!(" plannet · {}/{} done ", done, tasks.len());
        let hint = if self.editing.is_some() {
            " enter:save  esc:cancel "
        } else {
            " a:add  e:edit  space:done  n:notes  tab:focus  s:save  q:quit "
        };

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(border))
            .title(Span::styled(
                title,
                Style::new().fg(accent).add_modifier(Modifier::BOLD),
            ))
            .title_bottom(Span::styled(hint, Style::new().fg(Color::Gray)))
            .padding(Padding::horizontal(1)); // breathing room from the border

        // Empty state: a friendly, dimmed, centered prompt instead of a blank box.
        if tasks.is_empty() {
            let msg = Paragraph::new("No tasks yet — press 'a' to add one")
                .alignment(Alignment::Center)
                .style(Style::new().add_modifier(Modifier::DIM))
                .block(block);
            frame.render_widget(msg, area);
            return;
        }

        // Which row, if any, shows the live edit buffer instead of its title.
        let editing_index = self.editing.as_ref().and(self.state.selected());
        let buffer = self.editing.as_deref().unwrap_or("");

        let items: Vec<ListItem> = tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let checkbox = if task.done { "✓" } else { "☐" };
                if Some(i) == editing_index {
                    // Show the live buffer plus a block cursor (█) instead of the
                    // saved title, so typing is visible in place.
                    ListItem::new(format!("{checkbox} {buffer}\u{2588}"))
                } else if task.done {
                    // Done tasks recede: green check, with the title dimmed and
                    // struck through so open work stands out.
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{checkbox} "), Style::new().fg(Color::Green)),
                        Span::styled(
                            task.title.clone(),
                            Style::new().add_modifier(Modifier::DIM | Modifier::CROSSED_OUT),
                        ),
                    ]))
                } else {
                    ListItem::new(format!("{checkbox} {}", task.title))
                }
            })
            .collect();

        let list = List::new(items)
            .block(block)
            // A light-touch selection: bold the row and mark it with an accent
            // arrow rather than a heavy reversed bar.
            .highlight_style(Style::new().fg(accent).add_modifier(Modifier::BOLD))
            .highlight_symbol("❯ ");

        // A *stateful* widget reads `state` to know what to highlight, and can
        // also update the scroll offset — hence `&mut`.
        frame.render_stateful_widget(list, area, &mut self.state);
    }
}
