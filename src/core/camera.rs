use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use lightyear::prelude::Replicated;
use lightyear::shared::replication::components::Controlled;

use crate::constants::CAMERA_VIEW_HEIGHT;
use crate::coop::components::LocalControlled;
use crate::gameplay::effects::screen_shake::{ScreenShake, ScreenShakeRequest};
use crate::gameplay::player::components::Player;
use crate::pvp::components::PvpLocalPlayer;
use crate::states::{AppState, GamePhase};

#[derive(Component)]
pub struct MainCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenShake>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                camera_follow_player
                    .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
            )
            .add_systems(
                Update,
                camera_follow_coop_local.run_if(in_state(AppState::CoopGame)),
            )
            .add_systems(
                Update,
                camera_follow_pvp_local.run_if(in_state(AppState::PvpGame)),
            )
            .add_systems(
                Update,
                apply_screen_shake
                    .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
            );
    }
}

pub fn setup_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.projection.scaling_mode = fullscreen_scaling_mode();
    commands.spawn((camera, MainCamera));
}

pub fn fullscreen_scaling_mode() -> ScalingMode {
    ScalingMode::FixedVertical(CAMERA_VIEW_HEIGHT)
}

pub fn camera_follow_player(
    player_q: Query<&GlobalTransform, With<Player>>,
    mut camera_q: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
    time: Res<Time>,
) {
    let Ok(player_tf) = player_q.get_single() else {
        return;
    };
    let Ok(mut camera_tf) = camera_q.get_single_mut() else {
        return;
    };

    let target = player_tf.translation().truncate();
    let current = camera_tf.translation.truncate();
    let lerp = 1.0 - (-10.0 * time.delta_seconds()).exp();
    let next = current.lerp(target, lerp);
    camera_tf.translation.x = next.x;
    camera_tf.translation.y = next.y;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_uses_fullscreen_vertical_view_height() {
        let ScalingMode::FixedVertical(height) = fullscreen_scaling_mode() else {
            panic!("camera should use a fixed vertical fullscreen view");
        };
        assert_eq!(height, CAMERA_VIEW_HEIGHT);
    }
}

pub fn camera_follow_pvp_local(
    player_q: Query<&GlobalTransform, With<PvpLocalPlayer>>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    time: Res<Time>,
) {
    let Ok(player_tf) = player_q.get_single() else {
        return;
    };
    let Ok(mut camera_tf) = camera_q.get_single_mut() else {
        return;
    };

    let target = player_tf.translation().truncate();
    let current = camera_tf.translation.truncate();
    let lerp = 1.0 - (-10.0 * time.delta_seconds()).exp();
    let next = current.lerp(target, lerp);
    camera_tf.translation.x = next.x;
    camera_tf.translation.y = next.y;
}

pub fn camera_follow_coop_local(
    player_q: Query<
        (&GlobalTransform, Option<&Replicated>, Option<&Controlled>),
        (
            With<crate::gameplay::player::components::Player>,
            With<LocalControlled>,
        ),
    >,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    time: Res<Time>,
) {
    let Some(player_tf) = player_q
        .iter()
        .max_by_key(|(_, replicated, controlled)| {
            (replicated.is_some() as i32) * 10 + (controlled.is_some() as i32) * 20
        })
        .map(|(tf, _, _)| tf)
    else {
        return;
    };
    let Ok(mut camera_tf) = camera_q.get_single_mut() else {
        return;
    };

    let target = player_tf.translation().truncate();
    let current = camera_tf.translation.truncate();
    let lerp = 1.0 - (-10.0 * time.delta_seconds()).exp();
    let next = current.lerp(target, lerp);
    camera_tf.translation.x = next.x;
    camera_tf.translation.y = next.y;
}

pub fn apply_screen_shake(
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    mut shake: ResMut<ScreenShake>,
    mut requests: EventReader<ScreenShakeRequest>,
    time: Res<Time>,
) {
    for req in requests.read() {
        shake.trigger(req.strength, req.duration);
    }

    let Ok(mut tf) = camera_q.get_single_mut() else {
        return;
    };
    let offset = shake.update(time.delta_seconds());
    tf.translation.x += offset.x;
    tf.translation.y += offset.y;
}
