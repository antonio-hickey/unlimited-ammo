mod error;
mod watcher;

/// Build and run the watcher for updates in the codebase
/// TODO:
///   * Write some tests and think about edge cases
///   * Implement a per project config for unlimited-ammo
///     so users can specify settings for seperate codebases.
///   * Implement a basic terminal user interface
///   * Refactor try_build_codebase params
///   * Debug/Verbose mode
fn main() -> Result<(), error::Error> {
    // Initialize the debug logger
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Build the watcher / reloadoor
    let mut watcher = watcher::WatcherBuilder::new()
        .set_watch_interval(2)
        .build()
        .inspect_err(|_| log::error!("failed to build project watcher"))?;

    // "Welcome" message per say
    println!("\x1b[1;32mUnlimited Ammo Enabled\x1b[0m (v0.1.0)");
    println!("Just code, I'll cover reloading!\n\n");

    // Perform an initial build
    // TODO: should we do a initial web build ?
    // TODO: I dont like boolean param for rust/web build, not intuitive
    watcher
        .try_build_codebase(false)
        .inspect_err(|_| log::error!("failed to run initial build on project"))?;

    // Start watching the codebase for changes
    watcher.start().inspect_err(|e| log::error!("Uh Oh: {e:?}"))
}
