mod cli;
mod config;
mod hyprland;
mod snow;

use clap::Parser;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::Application;

fn main() -> Result<(), iced_layershell::Error> {
    let args = cli::Args::parse();
    let mut config = config::load_config();
    config::apply_cli_overrides(&mut config, &args);

    snow::Waysnow::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((0, 0)),
            exclusive_zone: -1,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::None,
            events_transparent: true,
            ..Default::default()
        },
        flags: config,
        ..Default::default()
    })
}
