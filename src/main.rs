mod error;
mod watcher;

/// Build and run the watcher for updates in the codebase
/// TODO:
///   * Write some tests and think about edge cases
///   * Implement a per project config for unlimited-ammo
///     so users can specify settings for seperate codebases.
fn main() -> Result<(), error::Error> {
    // Build the watcher / reloadoor
    let mut watcher = watcher::WatcherBuilder::new()
        .set_watch_interval(2)
        .build()?;

    // "Welcome" message per say
    println!("\x1b[1;32mUnlimited Ammo Enabled\x1b[0m");
    println!("We'll cover rebuilding, you just code broo\n\n");

    // Perform an initial build
    // TODO: should we do a initial web build ?
    // TODO: I dont like boolean param for rust/web build, not intuitive
    watcher.try_build_codebase(false)?;

    // Start watching the codebase for changes
    match watcher.start() {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{e:?}");
            Err(e)
        }
    }
}
