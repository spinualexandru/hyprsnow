use crate::cli::Args;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct SnowConfig {
    pub intensity: u8,
    pub size_min: f32,
    pub size_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub drift: f32,
    pub max_opacity: f32,
}

#[derive(Debug, Clone)]
pub enum ConfigEvent {
    ConfigChanged(SnowConfig),
}

impl Default for SnowConfig {
    fn default() -> Self {
        Self {
            intensity: 3,
            size_min: 2.0,
            size_max: 5.0,
            speed_min: 30.0,
            speed_max: 80.0,
            drift: 20.0,
            max_opacity: 1.0,
        }
    }
}

pub fn get_config_path() -> Option<PathBuf> {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        });

    let config_file = config_home.join("hypr").join("hyprsnow.conf");
    if config_file.exists() {
        Some(config_file)
    } else {
        None
    }
}

pub fn load_config() -> SnowConfig {
    let path = match get_config_path() {
        Some(p) => p,
        None => return SnowConfig::default(),
    };

    let mut config = hyprlang::Config::new();
    if config.parse_file(&path).is_err() {
        return SnowConfig::default();
    }

    SnowConfig {
        intensity: config
            .get_int("general:intensity")
            .map(|v| v.clamp(1, 10) as u8)
            .unwrap_or(3),
        size_min: config
            .get_float("general:size_min")
            .map(|v| v as f32)
            .unwrap_or(2.0),
        size_max: config
            .get_float("general:size_max")
            .map(|v| v as f32)
            .unwrap_or(5.0),
        speed_min: config
            .get_float("general:speed_min")
            .map(|v| v as f32)
            .unwrap_or(30.0),
        speed_max: config
            .get_float("general:speed_max")
            .map(|v| v as f32)
            .unwrap_or(80.0),
        drift: config
            .get_float("general:drift")
            .map(|v| v as f32)
            .unwrap_or(20.0),
        max_opacity: config
            .get_float("general:max_opacity")
            .map(|v| (v as f32).clamp(0.0, 1.0))
            .unwrap_or(1.0),
    }
}

pub fn apply_cli_overrides(config: &mut SnowConfig, args: &Args) {
    if let Some(v) = args.intensity {
        config.intensity = v;
    }
    if let Some(v) = args.size_min {
        config.size_min = v;
    }
    if let Some(v) = args.size_max {
        config.size_max = v;
    }
    if let Some(v) = args.speed_min {
        config.speed_min = v;
    }
    if let Some(v) = args.speed_max {
        config.speed_max = v;
    }
    if let Some(v) = args.drift {
        config.drift = v;
    }
    if let Some(v) = args.max_opacity {
        config.max_opacity = v.clamp(0.0, 1.0);
    }
}

pub fn spawn_config_watcher() -> mpsc::Receiver<ConfigEvent> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let config_path = match get_config_path() {
            Some(p) => p,
            None => {
                eprintln!("hyprsnow: No config file found, hot reload disabled");
                return;
            }
        };

        let watch_dir = match config_path.parent() {
            Some(p) => p.to_path_buf(),
            None => return,
        };

        let config_filename = config_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("hyprsnow.conf")
            .to_string();

        let tx_clone = tx.clone();
        let last_reload = std::sync::Arc::new(std::sync::Mutex::new(Instant::now()));
        let last_reload_clone = last_reload.clone();
        let debounce_duration = Duration::from_millis(100);

        let mut watcher = match notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        // Check if this event is for our config file
                        let is_config_file = event
                            .paths
                            .iter()
                            .any(|p| p.file_name().and_then(|n| n.to_str()) == Some(&config_filename));

                        if is_config_file {
                            // Debounce: skip if we reloaded recently
                            let mut last = last_reload_clone.lock().unwrap();
                            if last.elapsed() > debounce_duration {
                                *last = Instant::now();
                                drop(last);
                                let new_config = load_config();
                                let _ = tx_clone.send(ConfigEvent::ConfigChanged(new_config));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("hyprsnow: Failed to create file watcher: {}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
            eprintln!("hyprsnow: Failed to watch config directory: {}", e);
            return;
        }

        // Keep thread alive - watcher is dropped when thread ends
        loop {
            thread::park();
        }
    });

    rx
}
