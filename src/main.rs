// Declare the modules that live in sibling files. `mod` is what actually pulls a
// file into the crate — without these lines the compiler never looks at them.
mod app;
mod notes;
mod storage;
mod subtasks;
mod task;
mod ui;

use clap::{Parser, Subcommand};

use crate::app::App;

/// Top-level CLI. With no subcommand we open the TUI; `init` sets up the list.
#[derive(Parser)]
#[command(name = "plannet", about = "TUI app for tracking tasks")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Init,
    Version,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?; // prettier panic / error reports

    match Cli::parse().command {
        Some(Command::Init) => init(),
        Some(Command::Version) => {
            println!("plannet {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        None => run(),
    }
}

/// `plannet init`: create the task list, reporting whether we made it or it was
/// already there. The filesystem work lives in `storage`; this just talks to the
/// user.
fn init() -> color_eyre::Result<()> {
    if storage::init()? {
        println!("Tasks list initialized at \"{}/\"", storage::TASKS_DIR);
    } else {
        println!("Already initialized at \"{}/\"", storage::TASKS_DIR);
    }
    Ok(())
}

/// `plannet` with no subcommand: open the TUI if a list exists, otherwise point
/// the user at `init`.
fn run() -> color_eyre::Result<()> {
    let path = storage::tasks_path();
    if !path.exists() {
        println!("No list.toml found, run plannet init to begin.");
        return Ok(());
    }

    let tasks = storage::load(&path)?;

    // `ratatui::run` sets up the terminal (raw mode, alternate screen) and
    // guarantees it gets restored even if our code panics. The closure receives
    // the live terminal handle.
    let mut app = App::new(tasks, path);
    ratatui::run(|terminal| app.run(terminal))?;
    Ok(())
}
