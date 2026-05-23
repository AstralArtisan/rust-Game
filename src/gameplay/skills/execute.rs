use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::core::assets::GameAssets;
use crate::core::events::DamageEvent;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::augment::tuning;
use crate::gameplay::combat::components::{ArcHitbox, DamageKind, Hitbox, Lifetime, Team};
use crate::gameplay::combat::projectiles;
use crate::gameplay::effects::particles;
use crate::gameplay::effects::screen_shake::ScreenShakeRequest;
use crate::gameplay::enemy::components::{Enemy, EnemyBuffState};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::player::combat::MeleeSlashEffect;
use crate::gameplay::player::components::{
    AttackPower, DashState, Energy, FacingDirection, Health, InvincibilityTimer, Player,
    PlayerDriveInput, PlayerSkillState, SkillSlot, SkillSlots, SkillType,
};

const BARRAGE_PROJECTILE_COUNT: usize = 14;

pub fn activate_skill_inputs(
    mut commands: Commands,
    data: Option<Res<GameDataRegistry>>,
    assets: Res<GameAssets>,
    mut shake_events: EventWriter<ScreenShakeRequest>,
    mut damage_events: EventWriter<DamageEvent>,
    enemy_q: Query<
        (Entity, &GlobalTransform, &Health),
        (With<Enemy>, Without<Player>, Without<Replicated>),
    >,
    mut player_q: Query<
        (
            Entity,
            &PlayerDriveInput,
            &mut Transform,
            &FacingDirection,
            &AttackPower,
            &mut Health,
            &mut Energy,
            &SkillSlots,
            &PlayerSkillState,
            &DashState,
            &mut InvincibilityTimer,
            Option<&AugmentInventory>,
        ),
        (With<Player>, Without<Enemy>, Without<Replicated>),
    >,
) {
    let Ok((
        player_e,
        input,
        mut player_tf,
        facing,
        attack_power,
        mut health,
        mut energy,
        slots,
        skill_state,
        dash,
        mut invincibility,
        inventory,
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
    let Some(energy_cost) = skill_energy_cost(data.as_deref(), skill, energy.current) else {
        return;
    };

    let player_pos = player_tf.translation.truncate();
    let direction = facing.0.try_normalize().unwrap_or(Vec2::X);

    match skill {
        SkillType::GroundSlam => {
            spawn_ground_slam_arc(
                &mut commands,
                &assets,
                player_e,
                player_pos,
                direction,
                attack_power.0 * 3.0,
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
        }
        SkillType::BladeDance => {
            spawn_blade_dance_skill(&mut commands, &assets, player_e, player_pos, attack_power.0);
            invincibility.timer = Timer::from_seconds(1.55, TimerMode::Once);
            invincibility.timer.reset();
            shake_events.send(ScreenShakeRequest {
                strength: 5.0,
                duration: 0.20,
            });
        }
        SkillType::ExecutionBlade => {
            let Some((target, target_pos)) = enemy_q
                .iter()
                .filter(|(_, _, target_health)| target_health.current > 0.0)
                .min_by(|(_, _, a), (_, _, b)| a.current.total_cmp(&b.current))
                .map(|(target, target_tf, _)| (target, target_tf.translation().truncate()))
            else {
                return;
            };

            let strike_dir = (target_pos - player_pos)
                .try_normalize()
                .unwrap_or(direction);
            let blink_pos = target_pos - strike_dir * 44.0;
            particles::spawn_dash_particles(&mut commands, &assets, player_pos);
            player_tf.translation.x = blink_pos.x;
            player_tf.translation.y = blink_pos.y;
            invincibility.timer = Timer::from_seconds(0.20, TimerMode::Once);
            invincibility.timer.reset();
            damage_events.send(DamageEvent {
                target,
                source: Some(player_e),
                amount: attack_power.0 * 5.0,
                knockback: strike_dir * 650.0,
                team: Team::Player,
                kind: DamageKind::PlayerSkill,
                is_crit: false,
            });
            particles::spawn_hit_particles(
                &mut commands,
                &assets,
                target_pos,
                Color::srgba(1.0, 0.14, 0.18, 0.95),
            );
            shake_events.send(ScreenShakeRequest {
                strength: 8.0,
                duration: 0.18,
            });
        }
        SkillType::BulletBarrage => {
            spawn_bullet_barrage_skill(
                &mut commands,
                &assets,
                player_e,
                player_pos,
                direction,
                attack_power.0,
                inventory,
            );
            shake_events.send(ScreenShakeRequest {
                strength: 4.0,
                duration: 0.16,
            });
        }
        SkillType::FrostField => {
            spawn_radial_skill_hitbox(
                &mut commands,
                &assets,
                player_e,
                player_pos,
                260.0,
                attack_power.0 * 1.5,
                "FrostFieldSkill",
                Color::srgba(0.45, 0.85, 1.0, 0.24),
            );
            shake_events.send(ScreenShakeRequest {
                strength: 4.0,
                duration: 0.12,
            });
        }
        SkillType::MeteorFall => {
            let target_pos = input.aim_world.unwrap_or(player_pos + direction * 180.0);
            spawn_radial_skill_hitbox(
                &mut commands,
                &assets,
                player_e,
                target_pos,
                300.0,
                attack_power.0 * 4.0,
                "MeteorFallSkill",
                Color::srgba(1.0, 0.42, 0.18, 0.30),
            );
            shake_events.send(ScreenShakeRequest {
                strength: 9.0,
                duration: 0.26,
            });
        }
        SkillType::WarCry => {
            spawn_radial_skill_hitbox(
                &mut commands,
                &assets,
                player_e,
                player_pos,
                220.0,
                attack_power.0 * 0.5,
                "WarCrySkill",
                Color::srgba(1.0, 0.84, 0.32, 0.20),
            );
            energy.current = (energy.current + 10.0).min(energy.max);
        }
        SkillType::LifeDrain => {
            spawn_radial_skill_hitbox(
                &mut commands,
                &assets,
                player_e,
                player_pos,
                240.0,
                attack_power.0,
                "LifeDrainSkill",
                Color::srgba(0.88, 0.18, 0.32, 0.26),
            );
            let drain_fraction = if inventory
                .map(|inv| inv.stacks(AugmentId::LifestealSlash))
                .unwrap_or(0)
                > 0
            {
                0.80
            } else {
                0.50
            };
            health.current = (health.current + attack_power.0 * drain_fraction).min(health.max);
        }
        SkillType::TimeRift => {
            spawn_radial_skill_hitbox(
                &mut commands,
                &assets,
                player_e,
                player_pos,
                420.0,
                attack_power.0 * 0.8,
                "TimeRiftSkill",
                Color::srgba(0.54, 0.36, 1.0, 0.24),
            );
            invincibility.timer = Timer::from_seconds(3.0, TimerMode::Once);
            invincibility.timer.reset();
            for (enemy, _, _) in &enemy_q {
                commands.entity(enemy).insert(EnemyBuffState {
                    speed_mult: 0.30,
                    cooldown_mult: 0.30,
                    timer: Timer::from_seconds(3.0, TimerMode::Once),
                });
            }
        }
    }
    energy.current = (energy.current - energy_cost).max(0.0);

    // BulletStorm: spawn a ring of projectiles on any finisher activation
    let storm_stacks = inventory
        .map(|inv| inv.stacks(AugmentId::BulletStorm))
        .unwrap_or(0);
    if storm_stacks > 0
        && let Some(data) = data.as_deref()
    {
        let count = tuning::bullet_storm_projectile_count(data, storm_stacks);
        let bullet_damage = attack_power.0 * 1.5;
        let bullet_speed = 400.0;
        for i in 0..count {
            let angle = std::f32::consts::TAU * i as f32 / count as f32;
            let dir = Vec2::new(angle.cos(), angle.sin());
            projectiles::spawn_player_projectile_with_kind(
                &mut commands,
                &assets,
                player_e,
                player_pos + dir * 16.0,
                dir * bullet_speed,
                bullet_damage,
                0.0,
                DamageKind::PlayerSkill,
            );
        }
        shake_events.send(ScreenShakeRequest {
            strength: 4.0,
            duration: 0.12,
        });
        crate::gameplay::effects::particles::spawn_burst_ring(&mut commands, &assets, player_pos);
    }
}

fn skill_energy_cost(
    data: Option<&GameDataRegistry>,
    skill: SkillType,
    current_energy: f32,
) -> Option<f32> {
    if let Some(config) = data.and_then(|value| value.skills.get(skill)) {
        let required = if config.consumes_all_energy {
            config.min_energy.max(config.energy_cost)
        } else {
            config.energy_cost
        };
        if current_energy + f32::EPSILON < required {
            return None;
        }
        return Some(if config.consumes_all_energy {
            current_energy
        } else {
            config.energy_cost
        });
    }

    let (fallback_cost, fallback_required) = match skill {
        SkillType::GroundSlam | SkillType::BulletBarrage | SkillType::WarCry => (60.0, 60.0),
        SkillType::BladeDance
        | SkillType::ExecutionBlade
        | SkillType::FrostField
        | SkillType::LifeDrain => (80.0, 80.0),
        SkillType::MeteorFall | SkillType::TimeRift => (current_energy, 80.0),
    };
    if current_energy + f32::EPSILON < fallback_required {
        None
    } else {
        Some(fallback_cost)
    }
}

fn spawn_ground_slam_arc(
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
        Name::new("GroundSlamSkillHitbox"),
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
        Name::new("GroundSlamSkillVisual"),
    ));
}

fn spawn_blade_dance_skill(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    attack_power: f32,
) {
    for i in 0..8 {
        let angle = std::f32::consts::TAU * i as f32 / 8.0;
        let dir = Vec2::new(angle.cos(), angle.sin());
        spawn_ground_slam_arc(commands, assets, owner, pos, dir, attack_power * 0.60);
    }
    crate::gameplay::effects::particles::spawn_burst_ring(commands, assets, pos);
}

fn spawn_bullet_barrage_skill(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    direction: Vec2,
    attack_power: f32,
    inventory: Option<&AugmentInventory>,
) {
    let extra_mult = inventory
        .map(|inv| inv.stacks(AugmentId::ExtraProjectile))
        .filter(|stacks| *stacks > 0)
        .map(|_| 1.5)
        .unwrap_or(1.0);
    let count = (BARRAGE_PROJECTILE_COUNT as f32 * extra_mult).round() as usize;
    for i in 0..count {
        let spread = if count <= 1 {
            0.0
        } else {
            -0.26 + 0.52 * (i as f32 / (count - 1) as f32)
        };
        let dir = Mat2::from_angle(spread).mul_vec2(direction);
        projectiles::spawn_player_projectile_with_kind(
            commands,
            assets,
            owner,
            pos + dir * 18.0,
            dir * 620.0,
            attack_power * 0.40,
            0.0,
            DamageKind::PlayerSkill,
        );
    }
}

fn spawn_radial_skill_hitbox(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    radius: f32,
    damage: f32,
    name: &'static str,
    color: Color,
) {
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(60.0)),
            sprite: Sprite {
                color,
                custom_size: Some(Vec2::splat(radius * 2.0)),
                ..default()
            },
            ..default()
        },
        Hitbox {
            owner: Some(owner),
            team: Team::Player,
            damage_kind: DamageKind::PlayerSkill,
            size: Vec2::splat(radius * 2.0),
            damage,
            knockback: 180.0,
            can_crit: false,
            crit_chance: 0.0,
            crit_multiplier: 1.0,
        },
        Lifetime(Timer::from_seconds(0.18, TimerMode::Once)),
        InGameEntity,
        Name::new(name),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::assets::{AudioHandles, TextureHandles};

    #[test]
    fn fallback_skill_energy_costs_match_phase3_tiers() {
        assert_eq!(
            skill_energy_cost(None, SkillType::GroundSlam, 60.0),
            Some(60.0)
        );
        assert_eq!(
            skill_energy_cost(None, SkillType::BulletBarrage, 59.9),
            None
        );
        assert_eq!(
            skill_energy_cost(None, SkillType::LifeDrain, 80.0),
            Some(80.0)
        );
        assert_eq!(skill_energy_cost(None, SkillType::MeteorFall, 79.9), None);
        assert_eq!(
            skill_energy_cost(None, SkillType::MeteorFall, 100.0),
            Some(100.0)
        );
    }

    #[test]
    fn activate_skill_inputs_system_params_are_disjoint() {
        let mut app = App::new();
        app.insert_resource(GameAssets {
            font: Handle::default(),
            textures: TextureHandles::default(),
            audio: AudioHandles::default(),
        });
        app.add_event::<DamageEvent>()
            .add_event::<ScreenShakeRequest>()
            .add_systems(Update, activate_skill_inputs);

        app.update();
    }
}
