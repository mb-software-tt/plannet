//! The application shell: its state (`App`), the event loop, and the global keys
//! that don't belong to any one pane (quit, save, focus, toggling notes).
//!
//! Rendering lives in the sibling [`ui`](crate::ui) module — it only reads
//! `App`'s state, so keeping it separate keeps this file focused on *driving*
//! the loop. Because `ui` is a separate module, the state it reads is exposed as
//! `pub(crate)` rather than left fully private.
//!
//! Each pane owns its own keys and edit state: the task list ([`TaskListPane`]),
//! notes ([`NotesPane`](crate::notes::NotesPane)) and subtasks
//! ([`SubtasksPane`](crate::subtasks::SubtasksPane)). The loop just routes each
//! key to the focused pane and falls back to the global keys below.

use std::path::PathBuf;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::notes::NotesPane;
use crate::subtasks::SubtasksPane;
use crate::task::{Task, TaskListPane};

/// Whether a focused pane handled a key. The shell routes each key to the
/// focused pane first; `Pass` means "not mine" and the shell falls back to its
/// own global keys (quit, focus, save, …).
pub enum Outcome {
    Consumed,
    Pass,
}

/// Which pane currently has focus. `Tab` moves focus between them; the focused
/// pane gets the accent border.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Focus {
    List,
    Subtasks,
    Notes,
}

pub struct App {
    pub(crate) tasks: Vec<Task>,
    /// The task list pane: owns the selection and any in-progress title edit.
    pub(crate) tasks_pane: TaskListPane,
    /// Whether the notes/refinement pane is visible. Toggled with `n`.
    pub(crate) show_notes: bool,
    /// Which pane has focus. Moved with `Tab`.
    pub(crate) focus: Focus,
    /// The notes pane's own state (its editor + input handling). Keys go here
    /// when the notes pane is focused.
    pub(crate) notes: NotesPane,
    /// The subtasks pane's own state (selection + input handling). Keys go here
    /// when the subtasks pane is focused.
    pub(crate) subtasks: SubtasksPane,
    /// Where to persist the list (`.plannet/list.toml`); used when saving.
    path: PathBuf,
}

impl App {
    /// Build the app from tasks loaded off disk and the path they came from.
    pub fn new(tasks: Vec<Task>, path: PathBuf) -> Self {
        Self {
            tasks,
            tasks_pane: TaskListPane::default(), // selects the first row on first draw
            show_notes: true,                    // pane visible by default
            focus: Focus::List,                  // start focused on the task list
            notes: NotesPane::default(),
            subtasks: SubtasksPane::default(),
            path,
        }
    }

    /// The main loop. `&mut self` means this method borrows the App mutably
    /// (it can change pane state). It runs until the user presses `q`.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        loop {
            // `draw` hands us a closure; we forward the frame to our own method.
            terminal.draw(|frame| self.draw(frame))?;

            // `event::read()` blocks until *something* happens. We only care
            // about key presses for now; `if let` pattern-matches just the
            // Event::Key case and ignores everything else (mouse, resize, ...).
            if let Event::Key(key) = event::read()? {
                // Ctrl-C saves and quits from *any* state, mirroring `q`. In raw
                // mode the terminal doesn't turn this into a SIGINT, so it just
                // arrives as a key event we have to handle ourselves — and we
                // check it before routing, so it fires even mid-edit.
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    self.persist()?; // commits an in-progress note, then saves
                    return Ok(());
                }

                // Route to the focused pane first. If it consumes the key, we're
                // done; otherwise fall through to the shell's global keys.
                let selected = self.tasks_pane.selected();
                let outcome = match self.focus {
                    Focus::List => self.tasks_pane.handle_key(key, &mut self.tasks),
                    Focus::Notes => {
                        let notes = selected.map(|i| &mut self.tasks[i].notes);
                        self.notes.handle_key(key, notes)
                    }
                    Focus::Subtasks => {
                        let subtasks = selected.map(|i| &mut self.tasks[i].subtasks);
                        self.subtasks.handle_key(key, subtasks)
                    }
                };
                if let Outcome::Consumed = outcome {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => {
                        self.persist()?; // commits an in-progress note, then saves
                        return Ok(()); // leave the loop
                    }
                    KeyCode::Char('n') => {
                        self.show_notes = !self.show_notes;
                        // A hidden pane can't hold focus; hand it back to the list.
                        if !self.show_notes {
                            self.focus = Focus::List;
                        }
                    }
                    // Move focus between panes, but only when there's a second
                    // pane to focus.
                    KeyCode::Tab if self.show_notes => {
                        self.focus = match self.focus {
                            Focus::List => Focus::Notes,
                            Focus::Notes => Focus::Subtasks,
                            Focus::Subtasks => Focus::List,
                        };
                    }
                    KeyCode::Char('s') => self.persist()?, // manual save
                    _ => {}                                // ignore any other key
                }
            }
        }
    }

    /// Commit any in-progress note edit, then write the list to disk. Every
    /// exit/save path goes through here so a note being typed is never lost.
    fn persist(&mut self) -> color_eyre::Result<()> {
        self.commit_notes();
        self.save()
    }

    /// Flush the notes pane's editor (if any) back into the selected task. A
    /// no-op when not editing a note.
    fn commit_notes(&mut self) {
        let notes = self.tasks_pane.selected().map(|i| &mut self.tasks[i].notes);
        self.notes.commit(notes);
    }

    /// Write the current task list back to `list.toml`.
    fn save(&self) -> color_eyre::Result<()> {
        crate::storage::save(&self.path, &self.tasks)
    }
}
