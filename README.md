# hyprsnow

A snow overlay for Wayland/Hyprland. Snowflakes fall across your screen, land on window titlebars and the screen bottom, then melt away.


https://github.com/user-attachments/assets/634c215b-2aa7-40e0-9172-222355349400

## Note
This is a project made for fun and not tested for performance or battery usage.

## Quickstart

```bash
cargo run
```

## Installation

```bash
cargo build --release
cp target/release/hyprsnow ~/.local/bin/
```
OR
```
cargo install hyprsnow
```
OR

```
cargo install --path .
```

## Usage

```bash
hyprsnow [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--intensity <1-10>` | Snow intensity (default: 3) |
| `--size-min <float>` | Minimum snowflake size in pixels (default: 2.0) |
| `--size-max <float>` | Maximum snowflake size in pixels (default: 5.0) |
| `--speed-min <float>` | Minimum fall speed in pixels/second (default: 30.0) |
| `--speed-max <float>` | Maximum fall speed in pixels/second (default: 80.0) |
| `--drift <float>` | Horizontal drift intensity, 0 = none, 30 = strong (default: 20.0) |

## Configuration

Create `~/.config/hypr/hyprsnow.conf`:

```conf
general {
    intensity = 5
    size_min = 2.0
    size_max = 5.0
    speed_min = 30.0
    speed_max = 80.0
    drift = 20.0
}
```

CLI arguments override config file values.

## Hyprland Integration

hyprsnow listens to Hyprland IPC events and updates window positions in real-time. Snowflakes will land on the top edge of your windows as you open, close, move, or resize them.

## Dependencies

- Hyprland
- Rust

