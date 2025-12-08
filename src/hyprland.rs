use hyprland::data::{Clients, Monitor, Monitors, Workspace, Workspaces};
use hyprland::event_listener::AsyncEventListener;
use hyprland::prelude::*;
use hyprland::shared::Address;
use std::sync::mpsc;
use std::thread;

#[derive(Clone)]
pub struct WindowRect {
    pub address: Address,
    pub x: f32,
    pub y: f32,
    pub width: f32,
}

#[derive(Clone, Debug)]
pub struct MonitorRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub has_fullscreen: bool,
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
    let active_workspace_id = match Workspace::get_active() {
        Ok(ws) => ws.id,
        Err(_) => return Vec::new(),
    };

    match Clients::get() {
        Ok(clients) => clients
            .iter()
            .filter(|c| c.workspace.id == active_workspace_id)
            .map(|c| WindowRect {
                address: c.address.clone(),
                x: c.at.0 as f32,
                y: c.at.1 as f32,
                width: c.size.0 as f32,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

pub fn get_monitors_with_fullscreen_state() -> Vec<MonitorRect> {
    let monitors = match Monitors::get() {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };

    let workspaces = Workspaces::get().ok();

    monitors
        .iter()
        .map(|monitor| {
            let has_fullscreen = workspaces
                .as_ref()
                .and_then(|ws| {
                    ws.iter()
                        .find(|w| w.id == monitor.active_workspace.id)
                        .map(|w| w.fullscreen)
                })
                .unwrap_or(false);

            MonitorRect {
                x: monitor.x as f32,
                y: monitor.y as f32,
                width: monitor.width as f32,
                height: monitor.height as f32,
                has_fullscreen,
            }
        })
        .collect()
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
