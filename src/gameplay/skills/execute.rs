use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::core::assets::GameAssets;
use crate::core::events::DamageEvent;
use crate::data::definitions::SkillConfig;
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
    let Some(cfg) = data.as_deref().and_then(|d| d.skills.get(skill)) else {
        return;
    };
    let Some(energy_cost) = skill_energy_cost(cfg, energy.current) else {
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
                attack_power.0 * cfg.damage_mult,
                cfg.knockback,
                cfg.aoe_radius,
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
            spawn_blade_dance_skill(
                &mut commands,
                &assets,
                player_e,
                player_pos,
                attack_power.0 * cfg.damage_mult,
                cfg.aoe_radius,
            );
            let inv_s = cfg.status("invincibility_s");
            if inv_s > 0.0 {
                invincibility.timer = Timer::from_seconds(inv_s, TimerMode::Once);
                invincibility.timer.reset();
            }
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
            let inv_s = cfg.status("invincibility_s");
            if inv_s > 0.0 {
                invincibility.timer = Timer::from_seconds(inv_s, TimerMode::Once);
                invincibility.timer.reset();
            }
            damage_events.send(DamageEvent {
                target,
                source: Some(player_e),
                amount: attack_power.0 * cfg.damage_mult,
                knockback: strike_dir * cfg.knockback,
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
                attack_power.0 * cfg.damage_mult,
                cfg.projectile_count as usize,
                cfg.projectile_speed,
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
                cfg.aoe_radius,
                attack_power.0 * cfg.damage_mult,
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
                cfg.aoe_radius,
                attack_power.0 * cfg.damage_mult,
                "MeteorFallSkill",
                Color::srgba(1.0, 0.42, 0.18, 0.30),
            );
            shake_events.send(ScreenShakeRequest {
                strength: 9.0,
                duration: 0.26,
            });
        }
        SkillType::WarCry => {
            if cfg.damage_mult > 0.0 {
                spawn_radial_skill_hitbox(
                    &mut commands,
                    &assets,
                    player_e,
                    player_pos,
                    cfg.aoe_radius,
                    attack_power.0 * cfg.damage_mult,
                    "WarCrySkill",
                    Color::srgba(1.0, 0.84, 0.32, 0.20),
                );
            }
            // TODO: apply WarCry attack/move/attack-speed buff for cfg.duration_s
            // once a PlayerBuff component is wired into combat/movement systems.
        }
        SkillType::LifeDrain => {
            spawn_radial_skill_hitbox(
                &mut commands,
                &assets,
                player_e,
                player_pos,
                cfg.aoe_radius,
                attack_power.0 * cfg.damage_mult,
                "LifeDrainSkill",
                Color::srgba(0.88, 0.18, 0.32, 0.26),
            );
            let mut drain_fraction = cfg.status("lifesteal_fraction");
            // Lifesteal Slash augment boosts the conversion ratio.
            if inventory
                .map(|inv| inv.stacks(AugmentId::LifestealSlash))
                .unwrap_or(0)
                > 0
            {
                drain_fraction = (drain_fraction + 0.30).min(1.0);
            }
            health.current = (health.current + attack_power.0 * drain_fraction).min(health.max);
        }
        SkillType::TimeRift => {
            spawn_radial_skill_hitbox(
                &mut commands,
                &assets,
                player_e,
                player_pos,
                cfg.aoe_radius,
                attack_power.0 * cfg.damage_mult,
                "TimeRiftSkill",
                Color::srgba(0.54, 0.36, 1.0, 0.24),
            );
            let inv_s = cfg.status("invincibility_s");
            if inv_s > 0.0 {
                invincibility.timer = Timer::from_seconds(inv_s, TimerMode::Once);
                invincibility.timer.reset();
            }
            let slow = cfg.status("slow");
            let buff_s = if cfg.duration_s > 0.0 {
                cfg.duration_s
            } else {
                3.0
            };
            if slow > 0.0 {
                let speed_mult = (1.0 - slow).max(0.0);
                let cooldown_mult = (1.0 - slow).max(0.05);
                for (enemy, _, _) in &enemy_q {
                    commands.entity(enemy).insert(EnemyBuffState {
                        speed_mult,
                        cooldown_mult,
                        timer: Timer::from_seconds(buff_s, TimerMode::Once),
                    });
                }
            }
            // TODO: apply TimeRift attack-speed buff to player for buff_s.
        }
    }
    energy.current = (energy.current - energy_cost).max(0.0);

    // BulletStorm augment: spawn a ring of projectiles on any finisher activation.
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

fn skill_energy_cost(cfg: &SkillConfig, current_energy: f32) -> Option<f32> {
    let required = if cfg.consumes_all_energy {
        cfg.min_energy.max(cfg.energy_cost)
    } else {
        cfg.energy_cost
    };
    if current_energy + f32::EPSILON < required {
        return None;
    }
    Some(if cfg.consumes_all_energy {
        current_energy
    } else {
        cfg.energy_cost
    })
}

fn spawn_ground_slam_arc(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    direction: Vec2,
    damage: f32,
    knockback: f32,
    radius: f32,
) {
    let arc_size = Vec2::splat(radius * 1.2);
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(60.0)),
            sprite: Sprite {
                color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                custom_size: Some(arc_size),
                ..default()
            },
            ..default()
        },
        Hitbox {
            owner: Some(owner),
            team: Team::Player,
            damage_kind: DamageKind::PlayerSkill,
            size: arc_size,
            damage,
            knockback,
            can_crit: false,
            crit_chance: 0.0,
            crit_multiplier: 1.0,
        },
        ArcHitbox {
            origin: pos,
            direction,
            radius,
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
                translation: (pos + direction * (radius * 0.46)).extend(61.0),
                rotation: Quat::from_rotation_z(direction.y.atan2(direction.x)),
                scale: Vec3::new(2.6, 2.1, 1.0),
            },
            sprite: Sprite {
                color: Color::srgba(0.86, 1.0, 0.98, 0.92),
                custom_size: Some(Vec2::new(radius * 0.9, radius * 0.6)),
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
    damage_per_arc: f32,
    radius: f32,
) {
    for i in 0..8 {
        let angle = std::f32::consts::TAU * i as f32 / 8.0;
        let dir = Vec2::new(angle.cos(), angle.sin());
        spawn_ground_slam_arc(
            commands,
            assets,
            owner,
            pos,
            dir,
            damage_per_arc,
            0.0,
            radius,
        );
    }
    crate::gameplay::effects::particles::spawn_burst_ring(commands, assets, pos);
}

fn spawn_bullet_barrage_skill(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    direction: Vec2,
    damage_per_bullet: f32,
    base_count: usize,
    projectile_speed: f32,
    inventory: Option<&AugmentInventory>,
) {
    let extra_mult = inventory
        .map(|inv| inv.stacks(AugmentId::ExtraProjectile))
        .filter(|stacks| *stacks > 0)
        .map(|_| 1.5)
        .unwrap_or(1.0);
    let count = (base_count as f32 * extra_mult).round() as usize;
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
            dir * projectile_speed,
            damage_per_bullet,
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

    fn cfg(energy_cost: f32, consumes_all: bool, min_energy: f32) -> SkillConfig {
        SkillConfig {
            skill: SkillType::GroundSlam,
            category: crate::data::definitions::SkillCategory::Melee,
            tier: crate::data::definitions::SkillTier::Light,
            title: String::new(),
            description: String::new(),
            energy_cost,
            cooldown_s: 0.0,
            consumes_all_energy: consumes_all,
            min_energy,
            damage_mult: 0.0,
            knockback: 0.0,
            aoe_radius: 0.0,
            duration_s: 0.0,
            tick_interval_s: 0.0,
            projectile_count: 0,
            projectile_speed: 0.0,
            status: Default::default(),
        }
    }

    #[test]
    fn skill_energy_cost_respects_min_and_consume_all() {
        assert_eq!(skill_energy_cost(&cfg(60.0, false, 0.0), 60.0), Some(60.0));
        assert_eq!(skill_energy_cost(&cfg(60.0, false, 0.0), 59.9), None);
        assert_eq!(
            skill_energy_cost(&cfg(80.0, true, 80.0), 100.0),
            Some(100.0)
        );
        assert_eq!(skill_energy_cost(&cfg(80.0, true, 80.0), 79.9), None);
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
