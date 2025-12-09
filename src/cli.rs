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

    /// Maximum snowflake opacity (0.0-1.0, default 1.0)
    #[arg(long)]
    pub max_opacity: Option<f32>,

    /// Path to custom snowflake image
    /// If not provided, default circle shape will be used
    /// Make sure the image has a transparent background (e.g., PNG format)
    #[arg(long, num_args(1..))]
    pub image_paths: Option<Vec<String>>,
}
