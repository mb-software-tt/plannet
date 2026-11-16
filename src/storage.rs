//! Persistence: loading and saving the task list as TOML.
//!
//! This module owns everything about *where* tasks live on disk and *how* they
//! are serialized, so the TUI (`app`) never touches the filesystem directly.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::task::Task;

/// The hidden per-folder directory, like `.git`.
pub const TASKS_DIR: &str = ".plannet";
/// The single list file inside it.
pub const LIST_FILE: &str = "list.toml";

/// On-disk shape of `list.toml`. The `task` field name controls the TOML key, so
/// the file is an array of tables:
///
/// ```toml
/// [[task]]
/// title = "..."
/// done = false
/// ```
///
/// `#[serde(default)]` means a file with no `[[task]]` entries (or a completely
/// empty file) deserializes to an empty `Vec` instead of erroring.
#[derive(Default, Serialize, Deserialize)]
struct TaskFile {
    #[serde(default)]
    task: Vec<Task>,
}

/// The path to `list.toml` relative to the current directory: `.plannet/list.toml`.
pub fn tasks_path() -> PathBuf {
    Path::new(TASKS_DIR).join(LIST_FILE)
}

/// Create the `.plannet/` directory and an empty `list.toml`. Returns `false`
/// (without touching anything) if the list already exists, so we never clobber
/// a user's tasks. This lives here so all knowledge of the on-disk layout stays
/// in one module.
pub fn init() -> color_eyre::Result<bool> {
    let path = tasks_path();
    if path.exists() {
        return Ok(false);
    }

    fs::create_dir_all(TASKS_DIR)?;
    fs::write(&path, "")?; // an empty file; `load` reads it as an empty list
    Ok(true)
}

/// Read and parse the task list. An empty file yields an empty `Vec`.
pub fn load(path: &Path) -> color_eyre::Result<Vec<Task>> {
    let contents = fs::read_to_string(path)?;
    let file: TaskFile = toml::from_str(&contents)?;
    Ok(file.task)
}

/// Serialize the task list and write it to `path`, replacing any existing file.
pub fn save(path: &Path, tasks: &[Task]) -> color_eyre::Result<()> {
    // `toml::to_string` needs an owned, serializable value. We borrow the tasks
    // into a temporary wrapper just long enough to serialize them.
    let file = TaskFile {
        task: tasks.to_vec(),
    };
    let contents = toml::to_string(&file)?;
    fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::Notes;

    #[test]
    fn round_trips_through_a_file() -> color_eyre::Result<()> {
        let path = std::env::temp_dir().join("tui-tasks-storage-test.toml");

        let tasks = vec![
            Task {
                title: "first".into(),
                done: true,
                notes: Notes::default(),
                subtasks: Vec::new(),
            },
            Task {
                title: "second".into(),
                done: false,
                notes: Notes::default(),
                subtasks: Vec::new(),
            },
        ];
        save(&path, &tasks)?;
        let loaded = load(&path)?;

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].title, "first");
        assert!(loaded[0].done);
        assert_eq!(loaded[1].title, "second");
        assert!(!loaded[1].done);

        fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn empty_file_is_empty_list() -> color_eyre::Result<()> {
        let path = std::env::temp_dir().join("tui-tasks-storage-empty.toml");
        fs::write(&path, "")?;

        assert!(load(&path)?.is_empty());

        fs::remove_file(&path)?;
        Ok(())
    }
}
