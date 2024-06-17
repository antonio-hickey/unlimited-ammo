use crate::error::Error;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

/// Reponsible for watching the project for updates
pub struct Watcher {
    /// How fast (in seconds) to check files for updates
    watch_interval: u8,
    /// A vector of filenames to ignore
    ignore_list: Vec<String>,
    /// Target files to watch for changes
    targets: HashMap<String, SystemTime>,
    /// The current build process for the codebase
    current_build_process: Option<std::process::Child>,
}
impl Watcher {
    /// Start watching the project for updates
    pub fn start(&mut self) -> Result<(), Error> {
        // Initial state of targets
        self.targets = self.try_get_targets()?;

        loop {
            std::thread::sleep(Duration::from_secs(self.watch_interval as u64));

            // Current state of targets
            let targets_current_state = self.try_get_targets()?;

            'targets_loop: for (target, target_modified_ts) in &targets_current_state {
                if self.targets.get(target).unwrap() != target_modified_ts {
                    println!("Updated: {target} at {:?}", target_modified_ts);
                    match self.try_build_codebase() {
                        Ok(_) => break 'targets_loop,
                        Err(_) => continue 'targets_loop,
                    }
                }
            }

            // Update initial state of targets to current state
            self.targets = targets_current_state;
        }
    }

    /// Try to get a hashmap of target names and their last modified time
    fn try_get_targets(&self) -> Result<HashMap<String, SystemTime>, Error> {
        let mut targets: HashMap<String, SystemTime> = HashMap::new();
        self.walk_codebase(".", &mut targets)?;
        Ok(targets)
    }

    fn walk_codebase(
        &self,
        dir_path: &str,
        targets: &mut HashMap<String, SystemTime>,
    ) -> Result<(), Error> {
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            let filename = entry.file_name().into_string()?;

            if self.is_valid_target(&filename) {
                if path.is_dir() && path.to_str().is_some() {
                    self.walk_codebase(path.to_str().unwrap(), targets)?;
                } else {
                    let modified_ts = Self::try_get_modified_ts(&path)?;
                    targets.insert(filename, modified_ts);
                }
            }
        }

        Ok(())
    }

    /// Check if a target is valid (not in the ignore list)
    fn is_valid_target(&self, filename: &str) -> bool {
        !self.ignore_list.contains(&filename.to_string())
    }

    /// Try to get a timestamp of a paths last modification
    fn try_get_modified_ts(path: &PathBuf) -> Result<SystemTime, Error> {
        let modified_ts = std::fs::metadata(path)?.modified()?;
        Ok(modified_ts)
    }

    /// Try to build the codebase
    fn try_build_codebase(&mut self) -> Result<(), Error> {
        // Make sure ONLY 1 build process is ever living
        // (kill old one before spawning new one)
        if let Some(ref mut old_build) = self.current_build_process {
            old_build.kill()?;
            self.current_build_process = None;
        }

        // Run a new build of the codebase
        match Command::new("sh").arg("-c").arg("cargo run").spawn() {
            Ok(build_process) => {
                // TODO: refactor this block *maybe*, not a fan of this nesting
                if let Some(ref mut build_process) = self.current_build_process {
                    if let Some(ref mut output_stream) = build_process.stderr {
                        let mut output = Vec::new();
                        output_stream.read_to_end(&mut output)?;
                        println!("{:?}", String::from_utf8_lossy(&output))
                    }
                }

                self.current_build_process = Some(build_process);

                Ok(())
            }
            Err(e) => {
                eprintln!("{e:?}");
                Err(Error::BuildFailed(e))
            }
        }
    }
}

/// Builder Pattern Struct for `Watcher`
pub struct WatcherBuilder {
    watch_interval: Option<u8>,
    ignore_list: Option<Vec<String>>,
}
impl WatcherBuilder {
    /// Initiate a Builder Pattern Struct for `Watcher`
    pub fn new() -> Self {
        WatcherBuilder {
            watch_interval: None,
            ignore_list: None,
        }
    }

    /// Set the watch interval (in seconds) of how fast to poll for changes
    /// NOTE: This is required to build `Watcher`
    pub fn set_watch_interval(mut self, seconds: u8) -> Self {
        self.watch_interval = Some(seconds);
        self
    }

    /// Set the list of files for the `Watcher` to ignore changes
    /// NOTE: This has a default list if not explicitly set
    pub fn set_ignore_list(mut self, files_to_ignore: Vec<String>) -> Self {
        self.ignore_list = Some(files_to_ignore);
        self
    }

    /// Set the default list of files for the `Watcher` to ignore changes
    fn set_default_ignore_list(mut self) -> Self {
        self.ignore_list = Some(Vec::from([
            String::from(".git"),
            String::from(".gitignore"),
            String::from("target"),
        ]));
        self
    }

    /// Finish building `Watcher`
    pub fn build(mut self) -> Result<Watcher, Error> {
        // invariant checks
        if self.watch_interval.is_none() {
            return Err(Error::WatchIntervalNotSet);
        }
        if self.ignore_list.is_none() {
            self = self.set_default_ignore_list();
        }

        // NOTE: unwraping here is safe due to the invariant checks above
        let watcher = Watcher {
            watch_interval: self.watch_interval.unwrap(),
            ignore_list: self.ignore_list.unwrap(),
            targets: HashMap::new(),
            current_build_process: None,
        };

        Ok(watcher)
    }
}
