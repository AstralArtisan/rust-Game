use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::coop::components::{
    CoopDashVisualState, CoopPhase, CoopSessionEntity, CoopSessionState, GhostState,
};
use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::augment::tuning;
use crate::gameplay::combat::components::{DamageKind, Hitbox, Lifetime, Team};
use crate::gameplay::effects::{afterimage, particles};
use crate::gameplay::map::InGameEntity;

use super::components::*;

pub fn player_dash_input_system(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    data: Res<GameDataRegistry>,
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
            let distance = base_distance
                * tuning::blink_profile(&data, blink_stacks)
                    .map(|profile| profile.distance_mult)
                    .unwrap_or(1.0);
            // Use very high speed + tiny duration to teleport in one frame
            dash.speed = distance / 0.016;
            dash.timer = Timer::from_seconds(0.016, TimerMode::Once);
        }

        let extra_invuln = tuning::extended_invuln_bonus(
            &data,
            inventory
                .map(|value| value.stacks(AugmentId::ExtendedInvuln))
                .unwrap_or(0),
        );
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

        // Blink visual: particles at origin and destination
        if blink_stacks > 0 {
            let from = tf.translation().truncate();
            let to = from + dash.dir * dash.speed * 0.016;
            crate::gameplay::effects::particles::spawn_blink_particles(
                &mut commands,
                &assets,
                from,
                to,
            );
        }

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
    data: Res<GameDataRegistry>,
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

        dash.timer.tick(time.delta());
        if dash.timer.just_finished() {
            // DashShield: grant shield on dash end
            if let Some(inv) = inventory.as_ref() {
                let shield_stacks = inv.stacks(AugmentId::DashShield);
                if let Some(profile) = tuning::dash_shield_profile(&data, shield_stacks) {
                    commands.entity(player_e).insert(
                        crate::gameplay::augment::effects::DashShieldBuff {
                            timer: Timer::from_seconds(profile.cooldown_s, TimerMode::Once),
                            charges: profile.charges,
                            break_damage_fraction: profile.break_damage_fraction,
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
        if dash.trail_timer.just_finished() {
            let trail_damage = tuning::dash_trail_damage_fraction(
                &data,
                inventory
                    .map(|value| value.stacks(AugmentId::DashTrail))
                    .unwrap_or(0),
            )
            .map(|fraction| attack_power.0 * fraction)
            .or_else(|| mods.dash_damage_trail.then_some(attack_power.0 * 0.45));
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
