mod error;
mod watcher;

/// Build and run the watcher for updates in the codebase
/// TODO:
///   * Write some tests and think about edge cases
fn main() -> Result<(), error::Error> {
    let mut watcher = watcher::WatcherBuilder::new()
        .set_watch_interval(2)
        .build()?;

    match watcher.start() {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{e:?}");
            Err(e)
        }
    }
}
