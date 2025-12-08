use crate::config::{ConfigEvent, SnowConfig, spawn_config_watcher};
use crate::hyprland::{
    MonitorRect, WindowRect, get_hyprland_windows, get_monitors_with_fullscreen_state,
    get_total_screen_bounds, spawn_event_listener,
};
use hyprland::shared::Address;
use iced::mouse::Cursor;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Subscription, Task, Theme};
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
            x: rng.random_range(0.0..width),
            y: rng.random_range(0.0..height),
            radius: rng.random_range(config.size_min..config.size_max),
            speed: rng.random_range(config.speed_min..config.speed_max),
            phase: rng.random_range(0.0..std::f32::consts::TAU),
            drift_amount: rng.random_range(0.0..config.drift),
            opacity: rng.random_range(0.7..1.0) * config.max_opacity,
            state: SnowState::Falling,
        }
    }

    fn reset(&mut self, width: f32, height: f32, config: &SnowConfig, rng: &mut impl Rng) {
        self.x = rng.random_range(0.0..width);
        self.y = rng.random_range(-self.radius..height);
        self.radius = rng.random_range(config.size_min..config.size_max);
        self.speed = rng.random_range(config.speed_min..config.speed_max);
        self.phase = rng.random_range(0.0..std::f32::consts::TAU);
        self.drift_amount = rng.random_range(0.0..config.drift);
        self.opacity = rng.random_range(0.7..1.0) * config.max_opacity;
        self.state = SnowState::Falling;
    }
}

pub struct Waysnow {
    snowflakes: Vec<Snowflake>,
    windows: Vec<WindowRect>,
    monitors: Vec<MonitorRect>,
    event_rx: mpsc::Receiver<crate::hyprland::HyprlandEvent>,
    config_rx: mpsc::Receiver<ConfigEvent>,
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
            let mon_x = monitor.x - self.offset_x;
            let mon_y = monitor.y - self.offset_y;

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

    fn get_valid_spawn_ranges(&self) -> Vec<(f32, f32)> {
        self.monitors
            .iter()
            .filter(|m| !m.has_fullscreen)
            .map(|m| {
                let mon_x = m.x - self.offset_x;
                (mon_x, mon_x + m.width)
            })
            .collect()
    }

    fn apply_config_change(&mut self, new_config: SnowConfig) {
        let mut rng = rand::rng();
        let old_count = self.config.intensity as usize * 50;
        let new_count = new_config.intensity as usize * 50;

        self.config = new_config;

        if new_count > old_count {
            let valid_x_ranges = self.get_valid_spawn_ranges();
            for _ in old_count..new_count {
                let mut flake = Snowflake::new(self.width, self.height, &self.config, &mut rng);
                if !valid_x_ranges.is_empty() {
                    let range = &valid_x_ranges[rng.random_range(0..valid_x_ranges.len())];
                    flake.x = rng.random_range(range.0..range.1);
                }
                self.snowflakes.push(flake);
            }
        } else if new_count < old_count {
            self.snowflakes.truncate(new_count);
        }
    }
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Tick(Instant),
}

/// Boot function - initializes the application state
pub fn boot(config: SnowConfig) -> (Waysnow, Task<Message>) {
    let mut rng = rand::rng();
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
    let config_rx = spawn_config_watcher();

    (
        Waysnow {
            snowflakes,
            windows,
            monitors,
            event_rx,
            config_rx,
            last_tick: Instant::now(),
            time: 0.0,
            offset_x: min_x,
            offset_y: min_y,
            width,
            height,
            config,
            cache: canvas::Cache::default(),
        },
        Task::none(),
    )
}

/// Update function - handles messages and updates state
pub fn update(state: &mut Waysnow, message: Message) -> Task<Message> {
    match message {
        Message::Tick(now) => {
            let dt = now.duration_since(state.last_tick).as_secs_f32();
            state.last_tick = now;
            state.time += dt;

            // Check for hyprland events (non-blocking)
            while let Ok(_event) = state.event_rx.try_recv() {
                state.windows = get_hyprland_windows();
                state.monitors = get_monitors_with_fullscreen_state();
            }

            // Check for config changes (non-blocking)
            while let Ok(ConfigEvent::ConfigChanged(new_config)) = state.config_rx.try_recv() {
                state.apply_config_change(new_config);
            }

            let mut rng = rand::rng();
            let melt_duration = 4.0;
            let valid_x_ranges = state.get_valid_spawn_ranges();

            for flake in &mut state.snowflakes {
                match &mut flake.state {
                    SnowState::Falling => {
                        flake.y += flake.speed * dt;
                        flake.x += (state.time + flake.phase).sin() * flake.drift_amount * dt;

                        if flake.x < 0.0 {
                            flake.x = state.width;
                        } else if flake.x > state.width {
                            flake.x = 0.0;
                        }

                        let flake_bottom = flake.y + flake.radius;
                        let mut landed = false;

                        for window in &state.windows {
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

                        if !landed && flake.y > state.height - flake.radius {
                            flake.y = state.height - flake.radius;
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
                                state.windows.iter().find(|w| &w.address == addr)
                            {
                                let expected_y = window.y - flake.radius;

                                if (flake.y - expected_y).abs() > 1.0
                                    || *offset_x < 0.0
                                    || *offset_x > window.width
                                {
                                    flake.state = SnowState::Falling;
                                    continue;
                                }

                                flake.x = window.x + *offset_x;
                            } else {
                                flake.state = SnowState::Falling;
                                continue;
                            }
                        }

                        *melt_timer += dt;
                        let melt_progress = *melt_timer / melt_duration;
                        flake.opacity = (1.0 - melt_progress).max(0.0) * 0.9 * state.config.max_opacity;

                        if *melt_timer >= melt_duration {
                            flake.reset(state.width, state.height, &state.config, &mut rng);
                            if !valid_x_ranges.is_empty() {
                                let range = &valid_x_ranges[rng.random_range(0..valid_x_ranges.len())];
                                flake.x = rng.random_range(range.0..range.1);
                            }
                        }
                    }
                }
            }

            state.cache.clear();
        }
        _ => {}
    }

    Task::none()
}

/// View function - renders the UI
pub fn view(state: &Waysnow) -> Element<'_, Message, Theme, Renderer> {
    Canvas::new(state)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Subscription function - sets up event subscriptions
pub fn subscription(_state: &Waysnow) -> Subscription<Message> {
    iced::time::every(Duration::from_millis(16)).map(Message::Tick)
}

impl canvas::Program<Message> for &Waysnow {
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
