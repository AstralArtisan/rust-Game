use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::combat::components::{
    ArcHitbox, DamageKind, Hitbox, Lifetime, Projectile, Team,
};
use crate::gameplay::combat::projectiles;
use crate::gameplay::effects::particles;
use crate::gameplay::effects::screen_shake::ScreenShakeRequest;
use crate::gameplay::enemy::components::Enemy;
use crate::gameplay::map::InGameEntity;
use crate::gameplay::player::combat::MeleeSlashEffect;
use crate::gameplay::player::components::{
    ActiveSkill, AttackPower, DashState, Energy, FacingDirection, InvincibilityTimer, Player,
    PlayerDriveInput, PlayerSkillState, SkillSlot, SkillSlots, SkillType,
};
use crate::utils::entity::safe_despawn_recursive;

const LOCK_ON_DURATION_S: f32 = 2.0;
const LOCK_ON_DAMAGE_MULT: f32 = 8.0;
const LIGHTNING_DASH_DISTANCE: f32 = 600.0;
const LIGHTNING_DASH_SPEED: f32 = 2000.0;

#[derive(Component, Debug, Clone, Copy)]
pub struct MarkedTarget;

#[derive(Component, Debug, Clone, Copy)]
pub struct SkillMarkerVisual {
    pub target: Entity,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct HomingProjectile {
    pub target: Entity,
    pub speed: f32,
    pub turn_rate: f32,
}

pub fn activate_skill_inputs(
    mut commands: Commands,
    data: Option<Res<GameDataRegistry>>,
    assets: Res<GameAssets>,
    mut shake_events: EventWriter<ScreenShakeRequest>,
    enemy_q: Query<(Entity, &GlobalTransform), (With<Enemy>, Without<Replicated>)>,
    mut player_q: Query<
        (
            Entity,
            &PlayerDriveInput,
            &GlobalTransform,
            &FacingDirection,
            &AttackPower,
            &mut Energy,
            &SkillSlots,
            &mut PlayerSkillState,
            &mut DashState,
            &mut InvincibilityTimer,
        ),
        (With<Player>, Without<Replicated>),
    >,
) {
    let Ok((
        player_e,
        input,
        player_tf,
        facing,
        attack_power,
        mut energy,
        slots,
        mut skill_state,
        mut dash,
        mut invincibility,
    )) = player_q.get_single_mut()
    else {
        return;
    };

    if skill_state.blocks_attacks() || dash.active {
        return;
    }

    let requested_slot = if input.skill_1_pressed {
        Some(SkillSlot::One)
    } else if input.skill_2_pressed {
        Some(SkillSlot::Two)
    } else if input.skill_3_pressed {
        Some(SkillSlot::Three)
    } else if input.skill_4_pressed {
        Some(SkillSlot::Four)
    } else {
        None
    };
    let Some(slot) = requested_slot else {
        return;
    };

    let slot_state = slots.state(slot);
    if !slot_state.unlocked {
        return;
    }
    let Some(skill) = slot_state.skill else {
        return;
    };
    if skill == SkillType::Relic {
        return;
    }

    let finisher_cost = data
        .as_deref()
        .map(|value| value.player.finisher_charge_cost)
        .unwrap_or(100.0);
    if energy.current + f32::EPSILON < finisher_cost {
        return;
    }

    let player_pos = player_tf.translation().truncate();
    let direction = facing.0.try_normalize().unwrap_or(Vec2::X);

    match skill {
        SkillType::SwordArc => {
            spawn_sword_arc_skill(
                &mut commands,
                &assets,
                player_e,
                player_pos,
                direction,
                attack_power.0 * 5.0,
            );
            shake_events.send(ScreenShakeRequest {
                strength: 6.0,
                duration: 0.16,
            });
            particles::spawn_hit_particles(
                &mut commands,
                &assets,
                player_pos + direction * 92.0,
                Color::srgba(0.84, 1.0, 0.98, 0.90),
            );
            energy.current = (energy.current - finisher_cost).max(0.0);
        }
        SkillType::MarkedHunt => {
            let mut marked_any = false;
            for (target, target_tf) in &enemy_q {
                commands.entity(target).insert(MarkedTarget);
                spawn_mark_indicator(
                    &mut commands,
                    &assets,
                    target,
                    target_tf.translation().truncate(),
                );
                marked_any = true;
            }
            if !marked_any {
                return;
            }
            skill_state.active = ActiveSkill::LockOn {
                timer: Timer::from_seconds(LOCK_ON_DURATION_S, TimerMode::Once),
            };
            energy.current = (energy.current - finisher_cost).max(0.0);
        }
        SkillType::LightningDash => {
            let duration_s = LIGHTNING_DASH_DISTANCE / LIGHTNING_DASH_SPEED;
            dash.activate_lightning(
                direction,
                LIGHTNING_DASH_SPEED,
                duration_s,
                attack_power.0 * 4.0,
                attack_power.0 * 2.0,
                100.0,
            );
            invincibility.timer = Timer::from_seconds(duration_s + 0.05, TimerMode::Once);
            invincibility.timer.reset();
            particles::spawn_dash_particles(&mut commands, &assets, player_pos);
            shake_events.send(ScreenShakeRequest {
                strength: 5.0,
                duration: 0.14,
            });
            energy.current = (energy.current - finisher_cost).max(0.0);
        }
        SkillType::Relic => {}
    }
}

pub fn advance_lock_on_mode(
    mut commands: Commands,
    assets: Res<GameAssets>,
    time: Res<Time>,
    mut shake_events: EventWriter<ScreenShakeRequest>,
    mut marker_visuals: Query<(Entity, &SkillMarkerVisual)>,
    marked_targets: Query<Entity, (With<MarkedTarget>, Without<Replicated>)>,
    mut player_q: Query<
        (
            Entity,
            &GlobalTransform,
            &AttackPower,
            &mut PlayerSkillState,
        ),
        (With<Player>, Without<Replicated>),
    >,
) {
    let Ok((player_e, player_tf, attack_power, mut skill_state)) = player_q.get_single_mut() else {
        return;
    };

    let ActiveSkill::LockOn { timer } = &mut skill_state.active else {
        return;
    };
    timer.tick(time.delta());
    if !timer.finished() {
        return;
    }

    let targets = marked_targets.iter().collect::<Vec<_>>();
    let projectile_count = targets.len().max(1) as f32;
    let per_projectile_damage = attack_power.0 * LOCK_ON_DAMAGE_MULT / projectile_count;
    for target in targets {
        commands.entity(target).remove::<MarkedTarget>();
        spawn_homing_skill_projectile(
            &mut commands,
            &assets,
            player_e,
            player_tf.translation().truncate(),
            target,
            per_projectile_damage,
        );
    }

    for (entity, _) in &mut marker_visuals {
        safe_despawn_recursive(&mut commands, entity);
    }

    skill_state.active = ActiveSkill::Idle;
    shake_events.send(ScreenShakeRequest {
        strength: 4.0,
        duration: 0.12,
    });
}

pub fn update_mark_indicators(
    mut commands: Commands,
    time: Res<Time>,
    target_q: Query<(&GlobalTransform, Option<&MarkedTarget>)>,
    mut marker_q: Query<(Entity, &SkillMarkerVisual, &mut Transform, &mut Sprite)>,
) {
    for (entity, marker, mut transform, mut sprite) in &mut marker_q {
        let Ok((target_tf, marked)) = target_q.get(marker.target) else {
            safe_despawn_recursive(&mut commands, entity);
            continue;
        };
        if marked.is_none() {
            safe_despawn_recursive(&mut commands, entity);
            continue;
        }

        let pulse = (time.elapsed_seconds() * 8.0).sin().abs();
        transform.translation = target_tf.translation() + Vec3::new(0.0, 28.0 + pulse * 6.0, 80.0);
        sprite.color = Color::srgba(1.0, 0.20 + pulse * 0.18, 0.22, 0.78 + pulse * 0.16);
        sprite.custom_size = Some(Vec2::splat(14.0 + pulse * 6.0));
    }
}

pub fn update_homing_projectiles(
    target_q: Query<&GlobalTransform, (With<Enemy>, Without<Replicated>)>,
    mut projectile_q: Query<
        (&HomingProjectile, &mut Projectile, &mut Transform),
        Without<Replicated>,
    >,
) {
    for (homing, mut projectile, mut transform) in &mut projectile_q {
        let Ok(target_tf) = target_q.get(homing.target) else {
            continue;
        };
        let current_dir = projectile.velocity.try_normalize().unwrap_or(Vec2::X);
        let desired_dir = (target_tf.translation().truncate() - transform.translation.truncate())
            .try_normalize()
            .unwrap_or(current_dir);
        let steer = homing.turn_rate.clamp(0.0, 1.0);
        let next_dir = current_dir
            .lerp(desired_dir, steer)
            .try_normalize()
            .unwrap_or(desired_dir);
        projectile.velocity = next_dir * homing.speed;
        transform.rotation = Quat::from_rotation_z(next_dir.y.atan2(next_dir.x));
    }
}

fn spawn_sword_arc_skill(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    direction: Vec2,
    damage: f32,
) {
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(60.0)),
            sprite: Sprite {
                color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                custom_size: Some(Vec2::new(240.0, 200.0)),
                ..default()
            },
            ..default()
        },
        Hitbox {
            owner: Some(owner),
            team: Team::Player,
            damage_kind: DamageKind::PlayerSkill,
            size: Vec2::new(240.0, 200.0),
            damage,
            knockback: 420.0,
            can_crit: false,
            crit_chance: 0.0,
            crit_multiplier: 1.0,
        },
        ArcHitbox {
            origin: pos,
            direction,
            radius: 200.0,
            half_angle_rad: std::f32::consts::FRAC_PI_2,
        },
        Lifetime(Timer::from_seconds(0.10, TimerMode::Once)),
        InGameEntity,
        Name::new("SwordArcSkillHitbox"),
    ));

    commands.spawn((
        SpriteBundle {
            texture: assets.textures.slash.clone(),
            transform: Transform {
                translation: (pos + direction * 92.0).extend(61.0),
                rotation: Quat::from_rotation_z(direction.y.atan2(direction.x)),
                scale: Vec3::new(2.6, 2.1, 1.0),
            },
            sprite: Sprite {
                color: Color::srgba(0.86, 1.0, 0.98, 0.92),
                custom_size: Some(Vec2::new(180.0, 120.0)),
                ..default()
            },
            ..default()
        },
        TextureAtlas {
            layout: assets.textures.slash_layout.clone(),
            index: 0,
        },
        MeleeSlashEffect {
            timer: Timer::from_seconds(0.22, TimerMode::Once),
            base_alpha: 0.92,
            base_scale: Vec3::new(2.6, 2.1, 1.0),
            frame_count: 9,
        },
        InGameEntity,
        Name::new("SwordArcSkillVisual"),
    ));
}

fn spawn_mark_indicator(commands: &mut Commands, assets: &GameAssets, target: Entity, pos: Vec2) {
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation((pos + Vec2::new(0.0, 28.0)).extend(80.0)),
            sprite: Sprite {
                color: Color::srgba(1.0, 0.24, 0.28, 0.82),
                custom_size: Some(Vec2::splat(14.0)),
                ..default()
            },
            ..default()
        },
        SkillMarkerVisual { target },
        InGameEntity,
        Name::new("MarkedTargetIndicator"),
    ));
}

fn spawn_homing_skill_projectile(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    target: Entity,
    damage: f32,
) {
    let entity = projectiles::spawn_player_projectile_with_kind(
        commands,
        assets,
        owner,
        pos,
        Vec2::new(1.0, 0.0) * 520.0,
        damage,
        0.0,
        DamageKind::PlayerSkill,
    );
    commands.entity(entity).insert(HomingProjectile {
        target,
        speed: 520.0,
        turn_rate: 0.24,
    });
}
