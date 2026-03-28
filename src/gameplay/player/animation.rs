use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::core::events::DamageEvent;

use super::components::*;

#[derive(Component, Debug, Clone)]
pub struct PlayerAnim {
    pub state: AnimationState,
    pub timer: Timer,
}

pub fn update_player_animation_state(
    time: Res<Time>,
    mut damage_events: EventReader<DamageEvent>,
    mut q: Query<
        (
            Entity,
            &PlayerDriveInput,
            &Velocity,
            &DashState,
            &Health,
            &mut PlayerAnim,
            Option<&mut AnimationState>,
        ),
        (With<Player>, Without<Replicated>),
    >,
) {
    let damaged = damage_events.read().map(|event| event.target).collect::<Vec<_>>();
    for (player_e, input, vel, dash, health, mut anim, anim_state) in &mut q {
        anim.timer.tick(time.delta());

        let next_state = if health.current <= 0.0 {
            AnimationState::Dead
        } else if anim.state == AnimationState::Hurt && !anim.timer.finished() {
            AnimationState::Hurt
        } else if damaged.contains(&player_e) {
            anim.timer = Timer::from_seconds(0.12, TimerMode::Once);
            anim.timer.reset();
            AnimationState::Hurt
        } else if dash.active {
            AnimationState::Dash
        } else if input.attack_held || input.ranged_held {
            AnimationState::Attack
        } else if vel.0.length_squared() > 1.0 {
            AnimationState::Move
        } else {
            AnimationState::Idle
        };

        anim.state = next_state;
        if let Some(mut anim_state) = anim_state {
            *anim_state = next_state;
        }
    }
}

pub fn animate_player_sprite(
    mut q: Query<(&PlayerAnim, &FacingDirection, &mut Sprite), (With<Player>, Without<Replicated>)>,
) {
    for (anim, facing, mut sprite) in &mut q {
        sprite.flip_x = facing.0.x < -0.15;
        sprite.color = match anim.state {
            AnimationState::Idle => Color::WHITE,
            AnimationState::Move => Color::srgb(0.98, 0.99, 1.0),
            AnimationState::Attack => Color::srgb(1.0, 0.95, 0.92),
            AnimationState::Dash => Color::srgb(0.84, 0.94, 1.0),
            AnimationState::Hurt => Color::srgb(1.0, 0.82, 0.82),
            AnimationState::Dead => Color::srgb(0.40, 0.40, 0.40),
        };
    }
}
