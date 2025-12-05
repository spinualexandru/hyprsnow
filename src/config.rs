use crate::cli::Args;
use std::path::PathBuf;

#[derive(Clone)]
pub struct SnowConfig {
    pub intensity: u8,
    pub size_min: f32,
    pub size_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub drift: f32,
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
        }
    }
}

fn get_config_path() -> Option<PathBuf> {
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
}
