//! The notes slice: free-form refinement text attached to a task.
//!
//! This slice owns three things: the data type (`Notes`), the transient editing
//! state and input handling (`NotesPane`), and how both are drawn. The shell
//! only knows to route keys to the focused pane and hand it a rectangle — it
//! doesn't know a note is edited with a multi-line text area, or that `Esc`
//! commits it. That's the point of the component split.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Padding, Paragraph, Wrap},
};
use ratatui_textarea::TextArea;
use serde::{Deserialize, Serialize};

use crate::app::Outcome;

/// The refinement notes on a single task. A thin wrapper over a `String` for
/// now, but having its own type means the notes pane can hang view + input off
/// it without `Task` knowing the details.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Notes {
    /// The note text. `#[serde(default)]` keeps older files (with no `body`
    /// key) loading as empty rather than erroring.
    #[serde(default)]
    pub body: String,
}

impl Notes {
    /// Whether there's anything to show — drives the empty-state placeholder.
    pub fn is_empty(&self) -> bool {
        self.body.trim().is_empty()
    }
}

/// The notes pane's interaction state — the component the shell routes keys to
/// when the notes pane is focused.
///
/// `editor` is `Some` only while a note is being edited; the multi-line
/// `TextArea` owns the live buffer and cursor during that time, so a half-typed
/// note never touches the `Notes` data until it's committed. `None` is read
/// mode.
#[derive(Default)]
pub struct NotesPane {
    editor: Option<TextArea<'static>>,
}

impl NotesPane {
    /// Handle a key while the notes pane has focus. The return value tells the
    /// shell whether the key was *consumed*: in edit mode every key is ours (the
    /// text area eats it; `Esc` commits), so globals like `q`/`Tab` never fire.
    /// In read mode only `i`/`Enter` (enter edit) are ours — everything else
    /// passes back for the shell to handle.
    pub fn handle_key(&mut self, key: KeyEvent, notes: Option<&mut Notes>) -> Outcome {
        if self.editor.is_some() {
            match key.code {
                // `Esc` leaves edit mode, *keeping* the text. Unlike the
                // single-line title/subtask inputs (where `Enter` saves and `Esc`
                // discards), `Enter` here inserts a newline, so `Esc` is the save.
                KeyCode::Esc => self.commit(notes),
                // Everything else — including `Enter` for a newline — goes to the
                // text area, which owns the cursor and buffer.
                _ => {
                    self.editor.as_mut().expect("editing").input(key);
                }
            }
            Outcome::Consumed
        } else {
            match key.code {
                KeyCode::Char('i') | KeyCode::Enter => match notes {
                    // Only a real selection has notes to edit; with none, let the
                    // key pass so the shell can do something else with it.
                    Some(notes) => {
                        self.editor = Some(editor_for(&notes.body));
                        Outcome::Consumed
                    }
                    None => Outcome::Pass,
                },
                _ => Outcome::Pass,
            }
        }
    }

    /// Commit any in-progress edit back into `notes`, returning to read mode. A
    /// no-op when not editing, so the shell can call it unconditionally on the
    /// way out.
    pub fn commit(&mut self, notes: Option<&mut Notes>) {
        if let Some(editor) = self.editor.take()
            && let Some(notes) = notes
        {
            notes.body = editor.lines().join("\n");
        }
    }

    /// Paint the pane: the live editor when editing, otherwise the saved note (or
    /// a dimmed prompt when empty / nothing selected). The shell passes the
    /// focus-adjusted `border` colour and the `accent` for the title.
    pub fn render(
        &self,
        notes: Option<&Notes>,
        area: Rect,
        frame: &mut Frame,
        border: Color,
        accent: Color,
    ) {
        // While editing, the text area carries its own (yellow) block, so we just
        // render it.
        if let Some(editor) = &self.editor {
            frame.render_widget(editor, area);
            return;
        }

        let block = pane_block(border, accent);
        let widget = match notes {
            Some(notes) if !notes.is_empty() => Paragraph::new(notes.body.clone())
                .wrap(Wrap { trim: false })
                .block(block),
            _ => Paragraph::new("No notes yet. Press 'i' to write one.")
                .style(Style::new().add_modifier(Modifier::DIM))
                .block(block),
        };
        frame.render_widget(widget, area);
    }
}

/// Build a text area seeded with the existing note body, styled to match the
/// rest of the UI. A yellow border signals edit mode at a glance.
fn editor_for(body: &str) -> TextArea<'static> {
    let lines: Vec<String> = if body.is_empty() {
        Vec::new()
    } else {
        body.lines().map(str::to_string).collect()
    };
    let mut editor = TextArea::new(lines);
    editor.set_block(pane_block(Color::Yellow, Color::Yellow));
    // Keep just the block cursor — don't highlight the whole cursor line, which
    // would clash with the otherwise minimal UI.
    editor.set_cursor_line_style(Style::default());
    editor
}

/// The bordered box for the notes pane in read mode.
fn pane_block(border: Color, accent: Color) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border))
        .title(Span::styled(
            " notes ",
            Style::new().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::horizontal(1))
}
