use crate::{error::Error, interface::Display};
use chrono::{DateTime, SecondsFormat, Utc};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    process::{Child, Command},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime},
};

/// Reponsible for watching the project for updates
pub struct Watcher {
    /// How fast (in seconds) to check files for updates
    watch_interval: u8,

    /// A vector of filenames to ignore
    ignore_list: Vec<String>,

    /// Target files to watch for changes
    targets: HashMap<String, SystemTime>,

    /// Currently running build process
    current_build_process: Arc<Mutex<Option<Child>>>,

    /// The list of log messages to display within the UI
    display: Arc<Mutex<Display>>,
}
impl Watcher {
    /// Start watching the project for updates
    pub fn start(&mut self) -> Result<(), Error> {
        // Initial state of targets
        self.targets = self.try_get_targets().inspect_err(|_| {
            self.log("failed to get initial state of target files");
        })?;

        // Run an intial build on start up
        //
        // TODO: This should also detect or have a config
        // option for doing an initial web build as well.
        self.log("running the initial build");
        self.try_build_codebase(false)?;

        loop {
            std::thread::sleep(Duration::from_secs(self.watch_interval as u64));

            // Current state of targets
            let targets_current_state = self.try_get_targets().inspect_err(|_| {
                self.log("failed to get current state of target files");
            })?;

            'targets_loop: for (target, target_modified_ts) in &targets_current_state {
                if self
                    .targets
                    .get(target)
                    .is_some_and(|target| target != target_modified_ts)
                {
                    self.log(&format!("update detected @ {target}"));

                    let need_to_build_web: bool = target.contains("/src/web/");
                    match self.try_build_codebase(need_to_build_web) {
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

    /// Go through each file in a codebase (obeys ignore list)
    ///
    /// TODO: This function seems complex and not very readable by
    /// a quick glance. Either find a way to make it more easily
    /// comprehensible or add comments explaining what it's doing.
    fn walk_codebase(
        &self,
        dir_path: &str,
        targets: &mut HashMap<String, SystemTime>,
    ) -> Result<(), Error> {
        for entry in std::fs::read_dir(dir_path).inspect_err(|_| {
            self.log(&format!("failed to read directory: {dir_path}"));
        })? {
            if let Ok(entry) = entry.inspect_err(|e| {
                self.log(&format!("failed to get entry: {e}"));
            }) {
                let filename = entry.file_name().into_string().inspect_err(|_| {
                    self.log("failed to parse entry name into string");
                })?;
                let path = entry.path();

                if self.is_valid_target(&filename) {
                    if path.is_dir() && path.to_str().is_some() {
                        // SAFETY: This unwrap is safe via the invariant check above
                        self.walk_codebase(path.to_str().unwrap(), targets)
                            .inspect_err(|_| {
                                self.log(&format!("failed to walk codebase at entry: {path:?}"));
                            })?;
                    } else {
                        let modified_ts = Self::try_get_modified_ts(&path).inspect_err(|_| {
                            self.log(&format!(
                                "failed to get last modified timestamp for path: {path:?}"
                            ));
                        })?;

                        if let Some(path) = path.to_str() {
                            targets.insert(path.to_string(), modified_ts);
                        }
                    }
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
    fn try_get_modified_ts(path: &std::path::PathBuf) -> Result<SystemTime, Error> {
        let modified_ts = std::fs::metadata(path)?.modified()?;
        Ok(modified_ts)
    }

    /// Handle building and running the codebase.
    //
    // TODO: This needs to be refactored and cleaned up.
    pub fn try_build_codebase(&mut self, need_to_build_web: bool) -> Result<(), Error> {
        // If there's already a build running then kill and reset it
        if let Ok(mut current_build_process) = self.current_build_process.lock() {
            if let Some(ref mut old_build) = current_build_process.as_mut() {
                let pid = old_build.id();

                old_build.kill().inspect_err(|_| {
                    self.log(&format!(
                        "failed to kill the previous (stale) running build: (PID: {pid})",
                    ));
                })?;

                *current_build_process = None;
            }
        }

        if need_to_build_web {
            // NOTE: No need to track this process, we implicitly wait for it's completion.
            match Command::new("sh")
                .arg("-c")
                // TODO: The web build tool should be configurable, I've been using
                // bun a lot more than npm personally and lot's of people use other
                // stuff like yarn, pnpm, deno, etc
                .arg("cd src/web && npm run build")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
            {
                Ok(build_process) => {
                    let output = build_process.wait_with_output().inspect_err(|e| {
                        self.log(&format!("failed to build web:\n {e}"));
                    })?;
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    let stderr_str = String::from_utf8_lossy(&output.stderr);

                    // TODO: Figure out a fancy way to display the difference
                    // between logs originating from stdout and stderr.
                    self.log(&stdout_str);
                    self.log(&stderr_str);
                }
                Err(e) => {
                    self.log(&format!("failed to run web build command: {e}"));
                    return Err(Error::BuildFailed(e));
                }
            }
        }

        // else build the rust codebase
        match Command::new("sh")
            .arg("-c")
            .arg("RUSTFLAGS=\"-Awarnings\" cargo run --color=always")
            .env("RUST_LOG_STYLE", "always")
            .env("RUST_TERM_STYLE", "always")
            .env("CARGO_TERM_COLOR", "always")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut build_process) => {
                // Read stdout and display them as logs
                let display = Arc::clone(&self.display);
                if let Some(stdout) = build_process.stdout.take() {
                    thread::spawn(move || {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            match line {
                                Ok(text) => {
                                    if let Ok(mut display) = display.lock() {
                                        display.add_log(text);
                                    }
                                }
                                Err(e) => {
                                    log::error!("Error reading child stdout: {e}");
                                    break;
                                }
                            }
                        }
                    });
                }

                // Read stderr and display them as logs
                let display = Arc::clone(&self.display);
                if let Some(stderr) = build_process.stderr.take() {
                    thread::spawn(move || {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            match line {
                                Ok(text) => {
                                    if let Ok(mut display) = display.lock() {
                                        display.add_log(text);
                                    }
                                }
                                Err(e) => {
                                    log::error!("Error reading child stderr: {e}");
                                    break;
                                }
                            }
                        }
                    });
                }

                // Store this process in case we need to kill it later
                if let Ok(mut current_build_process) = self.current_build_process.lock() {
                    *current_build_process = Some(build_process);
                }

                Ok(())
            }
            Err(e) => {
                self.log("failed to run rust build command");

                Err(Error::BuildFailed(e))
            }
        }
    }

    /// Format a log message with the datetime and that it's from this app.
    ///
    /// NOTE: This is ONLY for logs that originate from Unlimited Ammo, other
    /// log messages from the users app is already formatted.
    fn format_log_msg(msg: &str) -> String {
        // Format file update detected message
        let datetime: DateTime<Utc> = Utc::now();
        let datetime = datetime.to_rfc3339_opts(SecondsFormat::Secs, true);

        // NOTE: The weird escape codes wrapped around "Unlimited Ammo"
        // is ANSI color escape codes, specifically to make it green.
        format!("[{datetime} \x1b[32mUnlimited Ammo\x1b[0m]: {msg}")
    }

    /// Add a log message to be displayed within the UI.
    pub fn log(&self, msg: &str) {
        let msg = Self::format_log_msg(msg);

        if let Ok(mut display) = self.display.lock() {
            display.add_log(msg);
        }
    }
}

/// Builder Pattern Struct for `Watcher`
pub struct WatcherBuilder {
    /// How fast (in seconds) the file watcher should check for changes.
    watch_interval: Option<u8>,

    /// The files the file watcher should ignore.
    ignore_list: Option<Vec<String>>,

    /// The list of log messages to display within the UI.
    display: Option<Arc<Mutex<Display>>>,

    /// The currently running build process.
    current_build_process: Option<Arc<Mutex<Option<Child>>>>,
}
impl WatcherBuilder {
    /// Initiate a Builder Pattern Struct for `Watcher`
    pub fn new() -> Self {
        WatcherBuilder {
            current_build_process: None,
            watch_interval: None,
            ignore_list: None,
            display: None,
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
    pub fn _set_ignore_list(mut self, files_to_ignore: Vec<String>) -> Self {
        self.ignore_list = Some(files_to_ignore);
        self
    }

    /// Set the log display, this is where the log
    /// messages are displayed within the user interface.
    pub fn set_display(mut self, display: Arc<Mutex<Display>>) -> Self {
        self.display = Some(display);
        self
    }

    /// Set the build process for the watcher.
    ///
    /// NOTE: This will always be set as None on
    /// initiation, the reason we want to pass it
    /// it in like this rather than defaulting to
    /// None, is so we can share it amongst threads.
    pub fn set_build_process(mut self, build_process: Arc<Mutex<Option<Child>>>) -> Self {
        self.current_build_process = Some(build_process);
        self
    }

    /// Set the default list of files for the `Watcher` to ignore changes
    fn set_default_ignore_list(mut self) -> Self {
        self.ignore_list = Some(Vec::from([
            String::from(".git"),
            String::from(".gitignore"),
            String::from("target"),
            String::from("README.md"),
            String::from("dist"),
            String::from("node_modules"),
            String::from("tsconfig.tsbuildinfo"),
            String::from("tsconfig.node.tsbuildinfo"),
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
        if self.display.is_none() {
            return Err(Error::DisplayNotSet);
        }

        // NOTE: unwraping here is safe due to the invariant checks above
        let watcher = Watcher {
            watch_interval: self.watch_interval.unwrap(),
            ignore_list: self.ignore_list.unwrap(),
            targets: HashMap::new(),
            current_build_process: self.current_build_process.unwrap(),
            display: self.display.unwrap(),
        };

        Ok(watcher)
    }
}
