use hyprland::data::{Clients, Monitor, Monitors};
use hyprland::event_listener::AsyncEventListener;
use hyprland::prelude::*;
use std::sync::mpsc;
use std::thread;

#[derive(Clone)]
pub struct WindowRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
}

#[derive(Debug, Clone)]
pub enum HyprlandEvent {
    WindowsChanged,
}

pub fn get_screen_size() -> (f32, f32) {
    match Monitor::get_active() {
        Ok(monitor) => (monitor.width as f32, monitor.height as f32),
        Err(_) => {
            // Fallback: try to get first monitor
            match Monitors::get() {
                Ok(monitors) => monitors
                    .iter()
                    .next()
                    .map(|m| (m.width as f32, m.height as f32))
                    .unwrap_or((1920.0, 1080.0)),
                Err(_) => (1920.0, 1080.0),
            }
        }
    }
}

pub fn get_hyprland_windows() -> Vec<WindowRect> {
    match Clients::get() {
        Ok(clients) => clients
            .iter()
            .map(|c| WindowRect {
                x: c.at.0 as f32,
                y: c.at.1 as f32,
                width: c.size.0 as f32,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

pub fn spawn_event_listener() -> mpsc::Receiver<HyprlandEvent> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let mut event_listener = AsyncEventListener::new();

            let tx_clone = tx.clone();
            event_listener.add_window_opened_handler(move |_| {
                let tx = tx_clone.clone();
                Box::pin(async move {
                    let _ = tx.send(HyprlandEvent::WindowsChanged);
                })
            });

            let tx_clone = tx.clone();
            event_listener.add_window_closed_handler(move |_| {
                let tx = tx_clone.clone();
                Box::pin(async move {
                    let _ = tx.send(HyprlandEvent::WindowsChanged);
                })
            });

            let tx_clone = tx.clone();
            event_listener.add_window_moved_handler(move |_| {
                let tx = tx_clone.clone();
                Box::pin(async move {
                    let _ = tx.send(HyprlandEvent::WindowsChanged);
                })
            });

            let tx_clone = tx.clone();
            event_listener.add_active_window_changed_handler(move |_| {
                let tx = tx_clone.clone();
                Box::pin(async move {
                    let _ = tx.send(HyprlandEvent::WindowsChanged);
                })
            });

            let tx_clone = tx.clone();
            event_listener.add_workspace_changed_handler(move |_| {
                let tx = tx_clone.clone();
                Box::pin(async move {
                    let _ = tx.send(HyprlandEvent::WindowsChanged);
                })
            });

            let tx_clone = tx.clone();
            event_listener.add_fullscreen_state_changed_handler(move |_| {
                let tx = tx_clone.clone();
                Box::pin(async move {
                    let _ = tx.send(HyprlandEvent::WindowsChanged);
                })
            });

            let _ = event_listener.start_listener_async().await;
        });
    });

    rx
}
