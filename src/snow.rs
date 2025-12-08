use crate::config::SnowConfig;
use crate::hyprland::{
    MonitorRect, WindowRect, get_hyprland_windows, get_monitors_with_fullscreen_state,
    get_total_screen_bounds, spawn_event_listener,
};
use hyprland::shared::Address;
use iced::mouse::Cursor;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Subscription, Theme};
use iced_layershell::Application;
use iced_layershell::to_layer_message;
use rand::Rng;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Clone)]
enum SnowState {
    Falling,
    Landed {
        melt_timer: f32,
        window_addr: Option<Address>,
        offset_x: f32,
    },
}

struct Snowflake {
    x: f32,
    y: f32,
    radius: f32,
    speed: f32,
    phase: f32,
    drift_amount: f32,
    opacity: f32,
    state: SnowState,
}

impl Snowflake {
    fn new(width: f32, height: f32, config: &SnowConfig, rng: &mut impl Rng) -> Self {
        Self {
            x: rng.gen_range(0.0..width),
            y: rng.gen_range(0.0..height),
            radius: rng.gen_range(config.size_min..config.size_max),
            speed: rng.gen_range(config.speed_min..config.speed_max),
            phase: rng.gen_range(0.0..std::f32::consts::TAU),
            drift_amount: rng.gen_range(0.0..config.drift),
            opacity: rng.gen_range(0.7..1.0),
            state: SnowState::Falling,
        }
    }

    fn reset_at_top(&mut self, width: f32, config: &SnowConfig, rng: &mut impl Rng) {
        self.x = rng.gen_range(0.0..width);
        self.y = -self.radius;
        self.radius = rng.gen_range(config.size_min..config.size_max);
        self.speed = rng.gen_range(config.speed_min..config.speed_max);
        self.phase = rng.gen_range(0.0..std::f32::consts::TAU);
        self.drift_amount = rng.gen_range(0.0..config.drift);
        self.opacity = rng.gen_range(0.7..1.0);
        self.state = SnowState::Falling;
    }
}

pub struct Waysnow {
    snowflakes: Vec<Snowflake>,
    windows: Vec<WindowRect>,
    monitors: Vec<MonitorRect>,
    event_rx: mpsc::Receiver<crate::hyprland::HyprlandEvent>,
    last_tick: Instant,
    time: f32,
    offset_x: f32,
    offset_y: f32,
    width: f32,
    height: f32,
    config: SnowConfig,
    cache: canvas::Cache,
}

impl Waysnow {
    fn is_in_fullscreen_monitor(&self, x: f32, y: f32) -> bool {
        for monitor in &self.monitors {
            // Adjust monitor coords from global to overlay space
            let mon_x = monitor.x - self.offset_x;
            let mon_y = monitor.y - self.offset_y;

            // Hide snowflakes in the entire column above/within fullscreen monitors
            if monitor.has_fullscreen
                && x >= mon_x
                && x < mon_x + monitor.width
                && y < mon_y + monitor.height
            {
                return true;
            }
        }
        false
    }
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Tick(Instant),
}

impl Application for Waysnow {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = SnowConfig;

    fn new(config: Self::Flags) -> (Self, iced::Task<Self::Message>) {
        let mut rng = rand::thread_rng();
        let (min_x, min_y, max_x, max_y) = get_total_screen_bounds();
        let width = max_x - min_x;
        let height = max_y - min_y;
        let count = config.intensity as usize * 50;

        let snowflakes = (0..count)
            .map(|_| Snowflake::new(width, height, &config, &mut rng))
            .collect();

        let windows = get_hyprland_windows();
        let monitors = get_monitors_with_fullscreen_state();
        let event_rx = spawn_event_listener();

        (
            Self {
                snowflakes,
                windows,
                monitors,
                event_rx,
                last_tick: Instant::now(),
                time: 0.0,
                offset_x: min_x,
                offset_y: min_y,
                width,
                height,
                config,
                cache: canvas::Cache::default(),
            },
            iced::Task::none(),
        )
    }

    fn namespace(&self) -> String {
        String::from("hyprsnow")
    }

    #[allow(clippy::single_match)]
    fn update(&mut self, message: Self::Message) -> iced::Task<Self::Message> {
        match message {
            Message::Tick(now) => {
                let dt = now.duration_since(self.last_tick).as_secs_f32();
                self.last_tick = now;
                self.time += dt;

                // Check for hyprland events (non-blocking)
                while let Ok(_event) = self.event_rx.try_recv() {
                    self.windows = get_hyprland_windows();
                    self.monitors = get_monitors_with_fullscreen_state();
                }

                let mut rng = rand::thread_rng();
                let melt_duration = 4.0;

                // Precompute valid x ranges (monitors without fullscreen) for spawning
                let valid_x_ranges: Vec<(f32, f32)> = self
                    .monitors
                    .iter()
                    .filter(|m| !m.has_fullscreen)
                    .map(|m| {
                        let mon_x = m.x - self.offset_x;
                        (mon_x, mon_x + m.width)
                    })
                    .collect();

                for flake in &mut self.snowflakes {
                    match &mut flake.state {
                        SnowState::Falling => {
                            flake.y += flake.speed * dt;
                            flake.x += (self.time + flake.phase).sin() * flake.drift_amount * dt;

                            if flake.x < 0.0 {
                                flake.x = self.width;
                            } else if flake.x > self.width {
                                flake.x = 0.0;
                            }

                            let flake_bottom = flake.y + flake.radius;
                            let mut landed = false;

                            for window in &self.windows {
                                if flake.x >= window.x
                                    && flake.x <= window.x + window.width
                                    && flake_bottom >= window.y
                                    && flake.y < window.y + 10.0
                                {
                                    flake.y = window.y - flake.radius;
                                    flake.state = SnowState::Landed {
                                        melt_timer: 0.0,
                                        window_addr: Some(window.address.clone()),
                                        offset_x: flake.x - window.x,
                                    };
                                    landed = true;
                                    break;
                                }
                            }

                            if !landed && flake.y > self.height - flake.radius {
                                flake.y = self.height - flake.radius;
                                flake.state = SnowState::Landed {
                                    melt_timer: 0.0,
                                    window_addr: None,
                                    offset_x: 0.0,
                                };
                            }
                        }
                        SnowState::Landed {
                            melt_timer,
                            window_addr,
                            offset_x,
                        } => {
                            if let Some(addr) = window_addr {
                                if let Some(window) =
                                    self.windows.iter().find(|w| &w.address == addr)
                                {
                                    let expected_y = window.y - flake.radius;

                                    // If window moved vertically or snowflake outside width, fall
                                    if (flake.y - expected_y).abs() > 1.0
                                        || *offset_x < 0.0
                                        || *offset_x > window.width
                                    {
                                        flake.state = SnowState::Falling;
                                        continue;
                                    }

                                    // Follow horizontal movement
                                    flake.x = window.x + *offset_x;
                                } else {
                                    // Window was closed - start falling again
                                    flake.state = SnowState::Falling;
                                    continue;
                                }
                            }

                            *melt_timer += dt;
                            let melt_progress = *melt_timer / melt_duration;
                            flake.opacity = (1.0 - melt_progress).max(0.0) * 0.9;

                            if *melt_timer >= melt_duration {
                                flake.reset_at_top(self.width, &self.config, &mut rng);
                                // Spawn in non-fullscreen area if possible
                                if !valid_x_ranges.is_empty() {
                                    let range = &valid_x_ranges[rng.gen_range(0..valid_x_ranges.len())];
                                    flake.x = rng.gen_range(range.0..range.1);
                                }
                            }
                        }
                    }
                }

                self.cache.clear();
            }
            _ => {}
        }

        iced::Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Renderer> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn style(&self, _theme: &Self::Theme) -> iced_layershell::Appearance {
        iced_layershell::Appearance {
            background_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        }
    }
}

impl canvas::Program<Message> for Waysnow {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let geometry = self
            .cache
            .draw(renderer, bounds.size(), |frame: &mut Frame| {
                for flake in &self.snowflakes {
                    // Skip snowflakes on monitors with fullscreen apps
                    if self.is_in_fullscreen_monitor(flake.x, flake.y) {
                        continue;
                    }

                    let color = Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: flake.opacity,
                    };

                    let circle = Path::circle(Point::new(flake.x, flake.y), flake.radius);
                    frame.fill(&circle, color);
                }
            });

        vec![geometry]
    }
}
