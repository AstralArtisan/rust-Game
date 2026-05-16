#![allow(dead_code)]

use bevy::prelude::*;

pub const WINDOW_WIDTH: f32 = 1920.0;
pub const WINDOW_HEIGHT: f32 = 1080.0;
pub const WINDOW_CLEAR_COLOR: Color = Color::srgb(0.06, 0.07, 0.10);

#[allow(dead_code)]
pub const CAMERA_ZOOM: f32 = 1.0;
pub const CAMERA_VIEW_HEIGHT: f32 = 760.0;

pub const ROOM_HALF_WIDTH: f32 = 640.0;
pub const ROOM_HALF_HEIGHT: f32 = 360.0;

#[allow(dead_code)]
pub const PLAYER_RADIUS: f32 = 16.0;
#[allow(dead_code)]
pub const ENEMY_RADIUS: f32 = 14.0;
#[allow(dead_code)]
pub const PROJECTILE_RADIUS: f32 = 6.0;

pub const UI_Z: f32 = 1000.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_view_and_room_keep_room_visible() {
        assert!(ROOM_HALF_HEIGHT * 2.0 <= CAMERA_VIEW_HEIGHT);
        assert_eq!(ROOM_HALF_WIDTH * 2.0, 1280.0);
        assert_eq!(ROOM_HALF_HEIGHT * 2.0, 720.0);
    }
}
