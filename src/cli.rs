use clap::Parser;

#[derive(Parser, Clone)]
#[command(name = "hyprsnow")]
#[command(about = "Snow overlay for Wayland/Hyprland")]
pub struct Args {
    /// Snow intensity (1-10)
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=10))]
    pub intensity: Option<u8>,

    /// Minimum snowflake size in pixels
    #[arg(long)]
    pub size_min: Option<f32>,

    /// Maximum snowflake size in pixels
    #[arg(long)]
    pub size_max: Option<f32>,

    /// Minimum fall speed in pixels/second
    #[arg(long)]
    pub speed_min: Option<f32>,

    /// Maximum fall speed in pixels/second
    #[arg(long)]
    pub speed_max: Option<f32>,

    /// Horizontal drift intensity (0 = none, 30 = strong)
    #[arg(long)]
    pub drift: Option<f32>,
}
