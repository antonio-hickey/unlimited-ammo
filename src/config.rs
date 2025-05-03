use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub watcher: WatcherConfig,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            watcher: WatcherConfig {
                watch_interval: 3,
                ignore_list: Vec::from([
                    String::from(".git"),
                    String::from(".gitignore"),
                    String::from("target"),
                    String::from("README.md"),
                    String::from("dist"),
                    String::from("node_modules"),
                    String::from("tsconfig.tsbuildinfo"),
                    String::from("tsconfig.node.tsbuildinfo"),
                    String::from(".unlimited-ammo-config.toml"),
                ]),
            },
        }
    }
}

/// Watcher configuration, this allows you to customize
/// how unlimited-ammo watches your project for changes.
#[derive(Deserialize, Clone)]
pub struct WatcherConfig {
    #[serde(rename = "watch-interval")]
    /// How fast (in seconds) to check files for updates
    pub watch_interval: u8,

    #[serde(rename = "ignore-list")]
    /// A list of filenames for unlimited-ammo to ignore
    pub ignore_list: Vec<String>,
}
