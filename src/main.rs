mod error;
mod interface;
mod watcher;

use self::error::Error;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::stdout,
    process::Child,
    sync::{Arc, Mutex},
    thread,
};

/// Unlimited Ammo Version
pub static VERSION: &str = "v0.2.0";

fn main() -> Result<(), Error> {
    // Setup the terminal user interface
    let terminal = ratatui::init();
    execute!(stdout(), EnterAlternateScreen).expect("failed to enter alternate screen");

    // Create a thread safe instance of the display interface
    let display = Arc::new(Mutex::new(interface::Display::new()));
    let display_clone = Arc::clone(&display);

    // Spawn the watcher in a new thread
    // so it doesn't block the interface
    let build_process: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let build_process_clone = Arc::clone(&build_process);
    thread::spawn(move || {
        watcher::WatcherBuilder::new()
            .set_watch_interval(2)
            .set_build_process(build_process_clone)
            .set_display(display_clone)
            .build()
            .expect("Failed to build project watcher")
            .start()
            .expect("Watcher failed to start");
    });

    // Run the interface application
    let app_result = interface::App::new(display, build_process).run(terminal);
    execute!(stdout(), LeaveAlternateScreen).expect("failed to leave alternate screen");
    ratatui::restore();
    app_result
}
