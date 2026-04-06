use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::coop::components::{
    CoopDashVisualState, CoopPhase, CoopSessionEntity, CoopSessionState, GhostState,
};
use crate::core::assets::GameAssets;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::combat::components::{DamageKind, Hitbox, Lifetime, Team};
use crate::gameplay::effects::screen_shake::ScreenShakeRequest;
use crate::gameplay::effects::{afterimage, particles};
use crate::gameplay::map::InGameEntity;

use super::components::*;

pub fn player_dash_input_system(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut sfx_events: EventWriter<crate::core::events::SfxEvent>,
    session_q: Query<&CoopSessionState, With<CoopSessionEntity>>,
    mut q: Query<
        (
            &PlayerDriveInput,
            &GlobalTransform,
            &mut DashCooldown,
            &mut DashState,
            &FacingDirection,
            &mut InvincibilityTimer,
            &Handle<Image>,
            &Sprite,
            Option<&AugmentInventory>,
            Option<&GhostState>,
            Option<&mut CoopDashVisualState>,
        ),
        (With<Player>, Without<Replicated>),
    >,
) {
    let phase = session_q
        .get_single()
        .map(|session| session.phase)
        .unwrap_or(CoopPhase::None);
    for (
        input,
        tf,
        mut cd,
        mut dash,
        facing,
        mut inv,
        texture,
        sprite,
        inventory,
        ghost,
        dash_visual,
    ) in &mut q
    {
        cd.timer.tick(time.delta());
        if dash.active
            || !input.dash_pressed
            || !cd.timer.finished()
            || phase != CoopPhase::None
            || matches!(ghost, Some(GhostState::Ghost))
        {
            continue;
        }

        cd.timer.reset();
        dash.reset_to_base();
        dash.active = true;
        dash.timer.reset();
        dash.trail_timer.reset();
        dash.dir = if input.move_axis.length_squared() > 0.0 {
            input.move_axis.normalize()
        } else {
            facing.0
        };
        if let Some(mut dash_visual) = dash_visual {
            dash_visual.active = true;
            dash_visual.dir = dash.dir;
        }

        // Blink: instant teleport instead of velocity-based dash
        let blink_stacks = inventory
            .map(|value| value.stacks(AugmentId::Blink))
            .unwrap_or(0);
        if blink_stacks > 0 {
            let base_distance = dash.speed * dash.base_duration_s;
            let distance = if blink_stacks >= 2 {
                base_distance * 1.5
            } else {
                base_distance
            };
            // Use very high speed + tiny duration to teleport in one frame
            dash.speed = distance / 0.016;
            dash.timer = Timer::from_seconds(0.016, TimerMode::Once);
        }

        let extra_invuln = match inventory
            .map(|value| value.stacks(AugmentId::ExtendedInvuln))
            .unwrap_or(0)
        {
            2 => 0.25,
            1 => 0.15,
            _ => 0.0,
        };
        let dash_duration = if blink_stacks > 0 {
            0.016
        } else {
            dash.base_duration_s
        };
        inv.timer = Timer::from_seconds(dash_duration + extra_invuln, TimerMode::Once);
        sfx_events.send(crate::core::events::SfxEvent {
            kind: crate::core::events::SfxKind::Dash,
        });
        particles::spawn_dash_particles(&mut commands, &assets, tf.translation().truncate());

        afterimage::spawn_afterimage(
            &mut commands,
            texture.clone(),
            tf.translation().truncate(),
            sprite.color.with_alpha(0.45),
            sprite.custom_size.unwrap_or(Vec2::splat(32.0)),
            sprite.flip_x,
        );
    }
}

pub fn update_dash_state(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut shake_events: EventWriter<ScreenShakeRequest>,
    mut q: Query<
        (
            Entity,
            &GlobalTransform,
            &mut DashState,
            &Handle<Image>,
            &Sprite,
            &AttackPower,
            &RewardModifiers,
            Option<&AugmentInventory>,
            Option<&mut CoopDashVisualState>,
        ),
        (With<Player>, Without<Replicated>),
    >,
) {
    for (player_e, tf, mut dash, texture, sprite, attack_power, mods, inventory, dash_visual) in
        &mut q
    {
        if !dash.active {
            if let Some(mut dash_visual) = dash_visual {
                dash_visual.active = false;
            }
            continue;
        }

        let dash_mode = dash.mode;
        dash.timer.tick(time.delta());
        if dash.timer.just_finished() {
            let end_pos = tf.translation().truncate();
            if dash_mode == DashMode::LightningSkill && dash.burst_damage > 0.0 {
                spawn_dash_burst_hitbox(
                    &mut commands,
                    &assets,
                    player_e,
                    end_pos,
                    dash.burst_radius.max(100.0),
                    dash.burst_damage,
                );
                particles::spawn_dash_particles(&mut commands, &assets, end_pos);
                shake_events.send(ScreenShakeRequest {
                    strength: 8.0,
                    duration: 0.18,
                });
            }
            // DashShield: grant shield on dash end
            if let Some(inv) = inventory.as_ref() {
                let shield_stacks = inv.stacks(AugmentId::DashShield);
                if shield_stacks > 0 {
                    let duration = if shield_stacks >= 2 { 5.0 } else { 3.0 };
                    commands.entity(player_e).insert(
                        crate::gameplay::augment::effects::DashShieldBuff {
                            timer: Timer::from_seconds(duration, TimerMode::Once),
                        },
                    );
                }
            }
            dash.reset_to_base();
            if let Some(mut dash_visual) = dash_visual {
                dash_visual.active = false;
                dash_visual.dir = dash.dir;
            }
            continue;
        }
        if let Some(mut dash_visual) = dash_visual {
            dash_visual.active = true;
            dash_visual.dir = dash.dir;
        }

        dash.trail_timer.tick(time.delta());
        if dash_mode == DashMode::LightningSkill {
            if dash.trail_timer.just_finished() {
                spawn_dash_skill_hitbox(
                    &mut commands,
                    &assets,
                    player_e,
                    tf.translation().truncate(),
                    dash.impact_damage.max(attack_power.0 * 4.0),
                );
            }
        } else if dash.trail_timer.just_finished() {
            let trail_damage = match inventory
                .map(|value| value.stacks(AugmentId::DashTrail))
                .unwrap_or(0)
            {
                2 => Some(attack_power.0 * 0.70),
                1 => Some(attack_power.0 * 0.40),
                _ if mods.dash_damage_trail => Some(attack_power.0 * 0.45),
                _ => None,
            };
            if let Some(trail_damage) = trail_damage {
                spawn_dash_trail_hitbox(
                    &mut commands,
                    &assets,
                    player_e,
                    tf.translation().truncate() - dash.dir * 10.0,
                    trail_damage,
                );
            }
        }

        afterimage::spawn_afterimage(
            &mut commands,
            texture.clone(),
            tf.translation().truncate(),
            sprite.color.with_alpha(0.25),
            sprite.custom_size.unwrap_or(Vec2::splat(32.0)),
            sprite.flip_x,
        );
    }
}

fn spawn_dash_trail_hitbox(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    damage: f32,
) {
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(54.0)),
            sprite: Sprite {
                color: Color::srgba(0.40, 0.95, 1.0, 0.25),
                custom_size: Some(Vec2::splat(24.0)),
                ..default()
            },
            ..default()
        },
        Hitbox {
            owner: Some(owner),
            team: Team::Player,
            damage_kind: DamageKind::PlayerSkill,
            size: Vec2::splat(24.0),
            damage,
            knockback: 220.0,
            can_crit: false,
            crit_chance: 0.0,
            crit_multiplier: 1.0,
        },
        Lifetime(Timer::from_seconds(0.08, TimerMode::Once)),
        InGameEntity,
        Name::new("DashTrailHitbox"),
    ));
}

fn spawn_dash_skill_hitbox(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    damage: f32,
) {
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(54.0)),
            sprite: Sprite {
                color: Color::srgba(0.62, 0.95, 1.0, 0.22),
                custom_size: Some(Vec2::new(72.0, 48.0)),
                ..default()
            },
            ..default()
        },
        Hitbox {
            owner: Some(owner),
            team: Team::Player,
            damage_kind: DamageKind::PlayerSkill,
            size: Vec2::new(72.0, 48.0),
            damage,
            knockback: 280.0,
            can_crit: false,
            crit_chance: 0.0,
            crit_multiplier: 1.0,
        },
        Lifetime(Timer::from_seconds(0.05, TimerMode::Once)),
        InGameEntity,
        Name::new("LightningDashTrail"),
    ));
}

fn spawn_dash_burst_hitbox(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    radius: f32,
    damage: f32,
) {
    let size = Vec2::splat(radius * 2.0);
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(55.0)),
            sprite: Sprite {
                color: Color::srgba(0.80, 0.96, 1.0, 0.18),
                custom_size: Some(size),
                ..default()
            },
            ..default()
        },
        Hitbox {
            owner: Some(owner),
            team: Team::Player,
            damage_kind: DamageKind::PlayerSkill,
            size,
            damage,
            knockback: 340.0,
            can_crit: false,
            crit_chance: 0.0,
            crit_multiplier: 1.0,
        },
        Lifetime(Timer::from_seconds(0.08, TimerMode::Once)),
        InGameEntity,
        Name::new("LightningDashBurst"),
    ));
}
