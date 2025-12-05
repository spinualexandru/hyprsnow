use crate::config::SnowConfig;
use crate::hyprland::{get_hyprland_windows, get_screen_size, spawn_event_listener, WindowRect};
use hyprland::shared::Address;
use iced::mouse::Cursor;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Subscription, Theme};
use iced_layershell::to_layer_message;
use iced_layershell::Application;
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
    event_rx: mpsc::Receiver<crate::hyprland::HyprlandEvent>,
    last_tick: Instant,
    time: f32,
    width: f32,
    height: f32,
    config: SnowConfig,
    cache: canvas::Cache,
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
        let (width, height) = get_screen_size();
        let count = config.intensity as usize * 50;

        let snowflakes = (0..count)
            .map(|_| Snowflake::new(width, height, &config, &mut rng))
            .collect();

        let windows = get_hyprland_windows();
        let event_rx = spawn_event_listener();

        (
            Self {
                snowflakes,
                windows,
                event_rx,
                last_tick: Instant::now(),
                time: 0.0,
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

    fn update(&mut self, message: Self::Message) -> iced::Task<Self::Message> {
        match message {
            Message::Tick(now) => {
                let dt = now.duration_since(self.last_tick).as_secs_f32();
                self.last_tick = now;
                self.time += dt;

                // Check for hyprland events (non-blocking)
                while let Ok(_event) = self.event_rx.try_recv() {
                    self.windows = get_hyprland_windows();
                }

                let mut rng = rand::thread_rng();
                let melt_duration = 4.0;

                for flake in &mut self.snowflakes {
                    match &mut flake.state {
                        SnowState::Falling => {
                            flake.y += flake.speed * dt;
                            flake.x +=
                                (self.time + flake.phase).sin() * flake.drift_amount * dt;

                            if flake.x < 0.0 {
                                flake.x = self.width;
                            } else if flake.x > self.width {
                                flake.x = 0.0;
                            }

                            let flake_bottom = flake.y + flake.radius;
                            let mut landed = false;

                            for window in &self.windows {
                                if flake.x >= window.x && flake.x <= window.x + window.width {
                                    if flake_bottom >= window.y && flake.y < window.y + 10.0 {
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
        let geometry = self.cache.draw(renderer, bounds.size(), |frame: &mut Frame| {
            for flake in &self.snowflakes {
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
