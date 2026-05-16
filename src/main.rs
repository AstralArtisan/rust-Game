mod app;
mod constants;
mod coop;
mod core;
mod data;
mod gameplay;
mod prelude;
mod pvp;
mod states;
mod ui;
mod utils;

use bevy::prelude::*;
use bevy::window::WindowMode;

use crate::app::GamePlugin;
use crate::constants::{WINDOW_CLEAR_COLOR, WINDOW_HEIGHT, WINDOW_WIDTH};

fn main() {
    App::new()
        .insert_resource(ClearColor(WINDOW_CLEAR_COLOR))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(primary_window_settings()),
            ..default()
        }))
        .add_plugins(GamePlugin)
        .run();
}

fn primary_window_settings() -> Window {
    Window {
        title: "勇闯方块城".to_string(),
        mode: WindowMode::BorderlessFullscreen,
        resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
        resizable: true,
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_window_defaults_to_fullscreen_design_resolution() {
        let window = primary_window_settings();

        assert_eq!(window.mode, WindowMode::BorderlessFullscreen);
        assert_eq!(window.resolution.width(), WINDOW_WIDTH);
        assert_eq!(window.resolution.height(), WINDOW_HEIGHT);
    }
}
