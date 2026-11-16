# plannet

A small terminal task manager built with [Ratatui]. Tasks are stored as TOML in a
per-folder `.plannet/list.toml`, like `.git` is per-repo.

## For users

### Install & run

```bash
plannet init   # create .plannet/list.toml in the current directory
plannet        # open the TUI (or prompts you to run init first)
```

Each folder has its own task list. Run `init` once per folder; running `plannet`
without a list tells you to do so.

### Layout

The task list is on the left. The side column on the right shows the selected
task's **notes** (free-form refinement text) above its **subtasks** (a
checklist). Press `n` to hide or show the side column and `tab` to move focus
between panes.

### Keys

Global (when not typing):

| Key      | Action                                   |
| -------- | ---------------------------------------- |
| `tab`    | cycle focus: tasks → notes → subtasks    |
| `n`      | show/hide the notes + subtasks column    |
| `s`      | save                                     |
| `q`      | save and quit                            |
| `ctrl-c` | save and quit (works even while editing) |

Task list:

| Key              | Action              |
| ---------------- | ------------------- |
| `↑`/`k`, `↓`/`j` | move selection      |
| `space`          | toggle done         |
| `a`              | add a task          |
| `e`              | edit selected title |

Notes:

| Key           | Action                      |
| ------------- | --------------------------- |
| `i` / `enter` | start editing               |
| `esc`         | finish editing (keeps text) |

Subtasks:

| Key              | Action          |
| ---------------- | --------------- |
| `↑`/`k`, `↓`/`j` | move selection  |
| `space`          | toggle done     |
| `a`              | add a subtask   |

Changes are saved on quit; `s` saves manually mid-session. When typing a task or
subtask title, `enter` saves it and `esc` cancels. In notes, `enter` inserts a
newline and `esc` saves.

## For developers

### Build & run from source

```bash
cargo run -- init   # run the binary with the `init` subcommand
cargo run           # open the TUI
cargo build --release   # optimized binary at target/release/plannet
cargo test          # run the test suite
```

Tip: `cargo run` opens the TUI in the current directory and will create a real
`.plannet/` here — `cd` into a scratch dir to avoid that.

### Layout

| File              | Responsibility                                          |
| ----------------- | ------------------------------------------------------- |
| `src/main.rs`     | CLI parsing (clap) and command dispatch                 |
| `src/app.rs`      | app state, event loop, focus, and global keys           |
| `src/ui.rs`       | top-level layout: where each pane is drawn              |
| `src/task.rs`     | the `Task` model and the task list pane                 |
| `src/notes.rs`    | the `Notes` model and the notes pane                    |
| `src/subtasks.rs` | the `Subtask` model and the subtasks pane               |
| `src/storage.rs`  | loading/saving `.plannet/list.toml` (serde + toml)      |

Each pane owns its own keys, edit state, and rendering. `app.rs` routes each
key to the focused pane and falls back to the global keys when the pane doesn't
handle it.

On-disk format is an array of tables; an empty file is an empty list. `notes`
and `subtasks` are optional, so older files still load:

```toml
[[task]]
title = "Render a list with Ratatui"
done = false

[task.notes]
body = "Start with a plain List widget."

[[task.subtasks]]
title = "Add a selection highlight"
done = true
```

[Ratatui]: https://ratatui.rs

## License

Copyright (c) 2026 Marc Bhopalsingh Software <marcbhopalsingh.software@gmail.com>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE
