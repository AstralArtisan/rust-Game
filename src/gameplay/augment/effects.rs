use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use lightyear::prelude::Replicated;
use rand::Rng;

use crate::core::assets::GameAssets;
use crate::core::events::{DamageAppliedEvent, DamageEvent};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::combat::components::{
    ArcHitbox, DamageKind, Hitbox, Hurtbox, Projectile, Team,
};
use crate::gameplay::combat::projectiles::HitTargets;
use crate::gameplay::effects::particles;
use crate::gameplay::enemy::components::Enemy;
use crate::gameplay::player::components::{
    AttackPower, DashState, Energy, Health, Player, RewardModifiers,
};
use crate::utils::collision::{Aabb2, aabb_from_transform_size};

use super::data::{AugmentId, AugmentInventory};
use super::tuning;

#[derive(Component, Debug, Clone)]
pub struct ArmorBroken {
    pub damage_multiplier: f32,
    pub crit_taken_bonus: f32,
    pub timer: Timer,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct HomingProjectile {
    pub speed: f32,
    pub turn_rate: f32,
    pub search_radius: f32,
    pub snap_radius: f32,
}

impl HomingProjectile {
    pub fn from_stacks(data: &GameDataRegistry, stacks: u8, speed: f32) -> Self {
        let search_radius = tuning::homing_search_radius(data, stacks).max(0.0);
        Self {
            speed,
            turn_rate: tuning::homing_turn_rate(data, stacks),
            search_radius,
            snap_radius: tuning::homing_snap_radius(data, stacks)
                .max(0.0)
                .min(search_radius),
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct DashResetSpeedBuff {
    pub timer: Timer,
    pub move_speed_mult: f32,
}

#[derive(Clone, Copy)]
struct MeleeReflector {
    owner: Entity,
    augment_stacks: u8,
    aabb: Aabb2,
    arc: ArcHitbox,
}

pub fn dash_energy_system(
    data: Res<GameDataRegistry>,
    mut damage_events: EventReader<DamageAppliedEvent>,
    mut dash_hits: Local<HashMap<Entity, HashSet<Entity>>>,
    mut player_q: ParamSet<(
        Query<(Entity, &DashState), (With<Player>, Without<Replicated>)>,
        Query<
            (&DashState, Option<&AugmentInventory>, &mut Energy),
            (With<Player>, Without<Replicated>),
        >,
    )>,
) {
    let active_players: HashSet<Entity> = player_q
        .p0()
        .iter()
        .filter_map(|(player, dash)| dash.active.then_some(player))
        .collect();
    dash_hits.retain(|player, _| active_players.contains(player));

    // Collect events first to avoid borrow conflicts with ParamSet
    let relevant_events: Vec<_> = damage_events
        .read()
        .filter(|event| {
            event.kind == DamageKind::PlayerSkill
                && event.target_team == Some(Team::Enemy)
                && event.source.is_some()
        })
        .map(|event| (event.source.unwrap(), event.target))
        .collect();

    let mut p1 = player_q.p1();
    for (player, target) in relevant_events {
        let Ok((dash, inventory, mut energy)) = p1.get_mut(player) else {
            continue;
        };
        if !dash.active {
            continue;
        }

        let stacks = inventory
            .map(|value| value.stacks(AugmentId::DashEnergy))
            .unwrap_or(0);
        if stacks == 0 {
            continue;
        }

        let hit_set = dash_hits.entry(player).or_default();
        if !hit_set.insert(target) {
            continue;
        }

        let gain = tuning::dash_energy_gain(&data, stacks, hit_set.len());
        energy.current = (energy.current + gain).min(energy.max);
    }
}

pub fn armor_broken_tick_system(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut ArmorBroken), Without<Replicated>>,
) {
    for (entity, mut armor_broken) in &mut q {
        armor_broken.timer.tick(time.delta());
        if armor_broken.timer.finished() {
            commands.entity(entity).remove::<ArmorBroken>();
        }
    }
}

pub fn melee_reflect_system(
    mut commands: Commands,
    assets: Res<GameAssets>,
    data: Res<GameDataRegistry>,
    owner_mods: Query<&RewardModifiers>,
    owner_augments: Query<&AugmentInventory>,
    mut collision_sets: ParamSet<(
        Query<(&Hitbox, &GlobalTransform, Option<&ArcHitbox>), Without<Replicated>>,
        Query<
            (
                Entity,
                &mut Projectile,
                &mut Hitbox,
                &GlobalTransform,
                &mut Transform,
                &mut Sprite,
            ),
            Without<Replicated>,
        >,
    )>,
) {
    let mut reflectors = Vec::new();
    for (hitbox, hitbox_tf, arc) in &collision_sets.p0() {
        if hitbox.team != Team::Player || hitbox.damage_kind != DamageKind::PlayerMelee {
            continue;
        }
        let Some(arc) = arc.copied() else {
            continue;
        };
        let Some(owner) = hitbox.owner else {
            continue;
        };
        let augment_stacks = owner_augments
            .get(owner)
            .map(|inventory| inventory.stacks(AugmentId::Reflect))
            .unwrap_or(0);
        let mastery_unlocked = owner_mods
            .get(owner)
            .map(|mods| mods.melee_projectile_reflect_unlocked())
            .unwrap_or(false);
        if augment_stacks == 0 && !mastery_unlocked {
            continue;
        }

        reflectors.push(MeleeReflector {
            owner,
            augment_stacks,
            aabb: aabb_from_transform_size(hitbox_tf, hitbox.size),
            arc,
        });
    }

    for (entity, mut projectile, mut projectile_hitbox, projectile_tf, mut tf, mut sprite) in
        &mut collision_sets.p1()
    {
        if projectile.team != Team::Enemy {
            continue;
        }

        let projectile_proxy = Hurtbox {
            team: Team::Enemy,
            size: projectile_hitbox.size,
        };
        let Some(reflector) = reflectors.iter().find(|reflector| {
            hitbox_intersects_target(
                reflector.aabb,
                Some(reflector.arc),
                &projectile_proxy,
                projectile_tf,
            )
        }) else {
            continue;
        };

        let reflected_dir =
            projectile_reflect_direction(projectile.velocity, reflector.arc.direction);
        let reflected_speed = projectile.velocity.length().max(360.0);
        let damage_mult = tuning::reflect_damage_mult(&data, reflector.augment_stacks);

        projectile.velocity = reflected_dir * reflected_speed;
        projectile.team = Team::Player;
        projectile_hitbox.owner = Some(reflector.owner);
        projectile_hitbox.team = Team::Player;
        projectile_hitbox.damage *= damage_mult;
        projectile_hitbox.knockback = projectile_hitbox.knockback.max(260.0);
        projectile_hitbox.can_crit = false;
        projectile_hitbox.crit_chance = 0.0;
        projectile_hitbox.crit_multiplier = 1.0;

        tf.translation += (reflected_dir * 12.0).extend(0.0);
        tf.rotation = Quat::from_rotation_z(reflected_dir.y.atan2(reflected_dir.x));
        sprite.color = Color::srgb(0.86, 1.0, 0.52);
        sprite.custom_size = Some(Vec2::new(18.0, 9.0));
        if tuning::reflect_homing(&data, reflector.augment_stacks) {
            commands
                .entity(entity)
                .insert(HomingProjectile::from_stacks(&data, 3, reflected_speed));
        }

        particles::spawn_hit_particles(
            &mut commands,
            &assets,
            projectile_tf.translation().truncate(),
            Color::srgba(0.92, 1.0, 0.72, 0.9),
        );
    }
}

pub fn homing_projectile_system(
    enemy_q: Query<(Entity, &GlobalTransform), (With<Enemy>, Without<Replicated>)>,
    mut projectile_q: Query<
        (
            &HomingProjectile,
            &mut Projectile,
            &mut Transform,
            Option<&HitTargets>,
        ),
        Without<Replicated>,
    >,
) {
    for (homing, mut projectile, mut transform, hit_targets) in &mut projectile_q {
        if projectile.team != Team::Player {
            continue;
        }

        let projectile_pos = transform.translation.truncate();
        let current_dir = projectile.velocity.try_normalize().unwrap_or(Vec2::X);
        let mut best_target = None;
        let mut best_dist_sq = homing.search_radius * homing.search_radius;

        for (enemy, enemy_tf) in &enemy_q {
            if hit_targets.is_some_and(|targets| targets.set.contains(&enemy)) {
                continue;
            }
            let delta = enemy_tf.translation().truncate() - projectile_pos;
            let dist_sq = delta.length_squared();
            if dist_sq >= best_dist_sq {
                continue;
            }
            if let Some(dir) = delta.try_normalize() {
                best_target = Some(dir);
                best_dist_sq = dist_sq;
            }
        }

        let Some(target_dir) = best_target else {
            continue;
        };
        let next_dir = homing_next_direction(current_dir, target_dir, best_dist_sq, *homing);
        projectile.velocity = next_dir * homing.speed;
        transform.rotation = Quat::from_rotation_z(next_dir.y.atan2(next_dir.x));
    }
}

fn homing_next_direction(
    current_dir: Vec2,
    target_dir: Vec2,
    target_dist_sq: f32,
    homing: HomingProjectile,
) -> Vec2 {
    let target_dir = target_dir.try_normalize().unwrap_or(current_dir);
    let current_dir = current_dir.try_normalize().unwrap_or(target_dir);
    if homing.snap_radius > 0.0 && target_dist_sq <= homing.snap_radius * homing.snap_radius {
        return target_dir;
    }

    current_dir
        .lerp(target_dir, homing.turn_rate.clamp(0.0, 1.0))
        .try_normalize()
        .unwrap_or(target_dir)
}

pub fn chain_lightning_system(
    mut commands: Commands,
    assets: Res<GameAssets>,
    data: Res<GameDataRegistry>,
    mut damage_events: EventReader<DamageAppliedEvent>,
    mut damage_writer: EventWriter<DamageEvent>,
    player_augments: Query<&AugmentInventory, (With<Player>, Without<Replicated>)>,
    enemy_q: Query<(Entity, &GlobalTransform, &Health), (With<Enemy>, Without<Replicated>)>,
) {
    for event in damage_events.read() {
        if event.kind != DamageKind::PlayerRanged || event.target_team != Some(Team::Enemy) {
            continue;
        }
        let Some(player) = event.source else {
            continue;
        };
        let Ok(inventory) = player_augments.get(player) else {
            continue;
        };
        let stacks = inventory.stacks(AugmentId::ChainLightning);
        if stacks == 0 {
            continue;
        }

        let Some(profile) = tuning::chain_lightning_profile(&data, stacks) else {
            continue;
        };
        let mut struck = HashSet::from([event.target]);
        let mut from_pos = event.pos;

        for _ in 0..profile.jumps {
            let mut next_target = None;
            let mut best_dist_sq = 180.0_f32 * 180.0;
            for (enemy, enemy_tf, health) in &enemy_q {
                if struck.contains(&enemy) || health.current <= 0.0 {
                    continue;
                }
                let to_enemy = enemy_tf.translation().truncate() - from_pos;
                let dist_sq = to_enemy.length_squared();
                if dist_sq >= best_dist_sq {
                    continue;
                }
                next_target = Some((enemy, enemy_tf.translation().truncate(), to_enemy));
                best_dist_sq = dist_sq;
            }

            let Some((enemy, enemy_pos, to_enemy)) = next_target else {
                break;
            };
            struck.insert(enemy);

            crate::gameplay::effects::particles::spawn_lightning_segment(
                &mut commands,
                &assets,
                from_pos,
                enemy_pos,
            );

            from_pos = enemy_pos;

            damage_writer.send(DamageEvent {
                target: enemy,
                source: Some(player),
                amount: event.amount * profile.damage_fraction,
                knockback: to_enemy.normalize_or_zero() * 120.0,
                team: Team::Player,
                kind: DamageKind::Passive,
                is_crit: false,
            });
        }
    }
}

pub fn explosive_projectile_system(
    mut commands: Commands,
    assets: Res<GameAssets>,
    data: Res<GameDataRegistry>,
    mut damage_events: EventReader<DamageAppliedEvent>,
    mut damage_writer: EventWriter<DamageEvent>,
    player_augments: Query<&AugmentInventory, (With<Player>, Without<Replicated>)>,
    enemy_q: Query<(Entity, &GlobalTransform, &Health), (With<Enemy>, Without<Replicated>)>,
) {
    for event in damage_events.read() {
        if event.kind != DamageKind::PlayerRanged || event.target_team != Some(Team::Enemy) {
            continue;
        }
        let Some(player) = event.source else {
            continue;
        };
        let Ok(inventory) = player_augments.get(player) else {
            continue;
        };
        let Some(profile) =
            tuning::explosive_shot_profile(&data, inventory.stacks(AugmentId::Piercing))
        else {
            continue;
        };

        particles::spawn_explosion_ring(&mut commands, &assets, event.pos, profile.radius);
        particles::spawn_hit_particles_count(
            &mut commands,
            &assets,
            event.pos,
            Color::srgba(1.0, 0.52, 0.18, 0.9),
            14,
        );

        let radius_sq = profile.radius * profile.radius;
        for (enemy, enemy_tf, health) in &enemy_q {
            if enemy == event.target || health.current <= 0.0 {
                continue;
            }

            let to_enemy = enemy_tf.translation().truncate() - event.pos;
            if to_enemy.length_squared() > radius_sq {
                continue;
            }

            damage_writer.send(DamageEvent {
                target: enemy,
                source: Some(player),
                amount: event.amount * profile.damage_fraction,
                knockback: to_enemy.normalize_or_zero() * profile.knockback,
                team: Team::Player,
                kind: DamageKind::Passive,
                is_crit: false,
            });
        }
    }
}

pub fn thorns_system(
    mut commands: Commands,
    assets: Res<GameAssets>,
    data: Res<GameDataRegistry>,
    mut damage_events: EventReader<DamageAppliedEvent>,
    mut damage_writer: EventWriter<DamageEvent>,
    player_augments: Query<(&AugmentInventory, &AttackPower), (With<Player>, Without<Replicated>)>,
    source_q: Query<&GlobalTransform, Without<Replicated>>,
    target_q: Query<&GlobalTransform, Without<Replicated>>,
) {
    for event in damage_events.read() {
        if event.attacker_team != Team::Enemy || event.target_team != Some(Team::Player) {
            continue;
        }
        let Some(source) = event.source else {
            continue;
        };
        let Ok((inventory, attack_power)) = player_augments.get(event.target) else {
            continue;
        };
        let stacks = inventory.stacks(AugmentId::Thorns);
        if stacks == 0 {
            continue;
        }

        let reflected_damage = attack_power.0 * tuning::thorns_damage_fraction(&data, stacks);
        let knockback = match (target_q.get(event.target), source_q.get(source)) {
            (Ok(target_tf), Ok(source_tf)) => {
                (source_tf.translation().truncate() - target_tf.translation().truncate())
                    .normalize_or_zero()
                    * 90.0
            }
            _ => Vec2::ZERO,
        };

        damage_writer.send(DamageEvent {
            target: source,
            source: Some(event.target),
            amount: reflected_damage,
            knockback,
            team: Team::Player,
            kind: DamageKind::Passive,
            is_crit: false,
        });

        // Thorns visual
        if let Ok(source_tf) = source_q.get(source) {
            crate::gameplay::effects::particles::spawn_thorns_particles(
                &mut commands,
                &assets,
                source_tf.translation().truncate(),
            );
        }
    }
}

pub fn dash_reset_speed_buff_tick_system(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut DashResetSpeedBuff), Without<Replicated>>,
) {
    for (entity, mut buff) in &mut q {
        buff.timer.tick(time.delta());
        if buff.timer.finished() {
            commands.entity(entity).remove::<DashResetSpeedBuff>();
        }
    }
}

fn hitbox_intersects_target(
    hb_aabb: Aabb2,
    arc: Option<ArcHitbox>,
    hurtbox: &Hurtbox,
    target_tf: &GlobalTransform,
) -> bool {
    if let Some(arc) = arc {
        return arc_hitbox_intersects_target(arc, hurtbox, target_tf);
    }

    let target_aabb = aabb_from_transform_size(target_tf, hurtbox.size);
    hb_aabb.intersects(target_aabb)
}

fn arc_hitbox_intersects_target(
    arc: ArcHitbox,
    hurtbox: &Hurtbox,
    target_tf: &GlobalTransform,
) -> bool {
    let to_target = target_tf.translation().truncate() - arc.origin;
    let distance = to_target.length();
    let target_radius = hurtbox.size.length() * 0.35;

    if distance > arc.radius + target_radius {
        return false;
    }
    if distance <= target_radius {
        return true;
    }

    let Some(target_dir) = to_target.try_normalize() else {
        return true;
    };
    let facing = arc.direction.try_normalize().unwrap_or(Vec2::X);
    let dot = facing.dot(target_dir).clamp(-1.0, 1.0);
    let angle = dot.acos();
    let angular_padding = (target_radius / distance.max(target_radius + 0.001))
        .clamp(0.0, 0.95)
        .asin();

    angle <= arc.half_angle_rad + angular_padding
}

// --- Legendary augment components ---

/// Frozen: enemy cannot move for duration.
/// We store the original speed multiplier so we can restore it.
#[derive(Component, Debug, Clone)]
pub struct Frozen {
    pub timer: Timer,
    pub shatter_damage_bonus: f32,
}

/// DashShield: absorbs one hit, expires after duration.
#[derive(Component, Debug, Clone)]
pub struct DashShieldBuff {
    pub timer: Timer,
    pub charges: u8,
    pub break_damage_fraction: f32,
}

/// Phoenix: marks that the once-per-run revive has been used.
#[derive(Component, Debug, Clone, Copy)]
pub struct PhoenixUsed;

// --- Legendary augment systems ---

/// Freeze: on PlayerRanged hit, chance to freeze enemy.
pub fn freeze_system(
    mut commands: Commands,
    data: Res<GameDataRegistry>,
    mut damage_events: EventReader<DamageAppliedEvent>,
    player_augments: Query<&AugmentInventory, (With<Player>, Without<Replicated>)>,
    mut rng: ResMut<crate::utils::rng::GameRng>,
    existing_frozen: Query<(), (With<Frozen>, Without<Replicated>)>,
    shielded_q: Query<&crate::gameplay::enemy::components::ShieldedAffixState, Without<Replicated>>,
) {
    for event in damage_events.read() {
        if event.kind != DamageKind::PlayerRanged || event.target_team != Some(Team::Enemy) {
            continue;
        }
        let Some(player) = event.source else {
            continue;
        };
        let Ok(inventory) = player_augments.get(player) else {
            continue;
        };
        let stacks = inventory.stacks(AugmentId::Freeze);
        if stacks == 0 {
            continue;
        }
        // Already frozen, skip
        if existing_frozen.get(event.target).is_ok() {
            continue;
        }
        // Shielded elites with immune_freeze cannot be chilled.
        if shielded_q.get(event.target).is_ok_and(|s| s.immune_freeze) {
            continue;
        }
        let Some(profile) = tuning::freeze_profile(&data, stacks) else {
            continue;
        };
        if rng.gen_range_f32(0.0, 1.0) < profile.chance
            && let Some(mut ec) = commands.get_entity(event.target)
        {
            ec.insert(Frozen {
                timer: Timer::from_seconds(profile.duration_s, TimerMode::Once),
                shatter_damage_bonus: profile.shatter_bonus,
            });
        }
    }
}

/// Tick frozen timers, tint sprite blue while frozen, remove when expired.
pub fn tick_frozen_system(
    mut commands: Commands,
    assets: Res<GameAssets>,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Frozen, Option<&mut Sprite>, &GlobalTransform), Without<Replicated>>,
) {
    let mut rng = rand::thread_rng();
    for (entity, mut frozen, sprite, gtf) in &mut q {
        frozen.timer.tick(time.delta());
        if let Some(mut sprite) = sprite {
            sprite.color = Color::srgba(0.5, 0.7, 1.0, 1.0);
        }
        // Spawn occasional ice crystal particles
        if rng.gen_ratio(1, 8) {
            let pos = gtf.translation().truncate();
            let offset = Vec2::new(rng.gen_range(-12.0..12.0), rng.gen_range(-8.0..16.0));
            commands.spawn((
                SpriteBundle {
                    texture: assets.textures.white.clone(),
                    transform: Transform::from_translation((pos + offset).extend(52.0)),
                    sprite: Sprite {
                        color: Color::srgba(0.6, 0.85, 1.0, 0.6),
                        custom_size: Some(Vec2::splat(3.0)),
                        ..default()
                    },
                    ..default()
                },
                crate::gameplay::effects::particles::Particle {
                    velocity: Vec2::new(rng.gen_range(-10.0..10.0), rng.gen_range(15.0..30.0)),
                    lifetime: Timer::from_seconds(0.4, TimerMode::Once),
                },
                crate::gameplay::map::InGameEntity,
                Name::new("IceCrystal"),
            ));
        }
        if frozen.timer.finished() {
            commands.entity(entity).remove::<Frozen>();
        }
    }
}

/// DashShield: tick timer, remove when expired. Manage shield visual.
pub fn tick_dash_shield_system(
    mut commands: Commands,
    assets: Res<GameAssets>,
    time: Res<Time>,
    mut q: Query<(Entity, &mut DashShieldBuff), (With<Player>, Without<Replicated>)>,
    visual_q: Query<(Entity, &crate::gameplay::effects::particles::ShieldVisual)>,
) {
    for (entity, mut shield) in &mut q {
        shield.timer.tick(time.delta());
        if shield.timer.finished() {
            commands.entity(entity).remove::<DashShieldBuff>();
            for (vis, visual) in &visual_q {
                if visual.owner == entity {
                    crate::utils::entity::safe_despawn_recursive(&mut commands, vis);
                }
            }
        }
    }

    for (vis, visual) in &visual_q {
        if q.get(visual.owner).is_err() {
            crate::utils::entity::safe_despawn_recursive(&mut commands, vis);
        }
    }

    for (player_entity, _) in &mut q {
        let has_visual = visual_q
            .iter()
            .any(|(_, visual)| visual.owner == player_entity);
        if !has_visual {
            commands.entity(player_entity).with_children(|parent| {
                parent.spawn((
                    SpriteBundle {
                        texture: assets.textures.white.clone(),
                        transform: Transform::from_translation(Vec3::new(0.0, 0.0, -0.3)),
                        sprite: Sprite {
                            color: Color::srgba(0.4, 0.7, 1.0, 0.25),
                            custom_size: Some(Vec2::splat(64.0)),
                            ..default()
                        },
                        ..default()
                    },
                    crate::gameplay::effects::particles::ShieldVisual {
                        owner: player_entity,
                    },
                    Name::new("ShieldVisual"),
                ));
            });
        }
    }
}

/// Phoenix: intercept player death by checking health <= 0 before DeathEvent is processed.
/// Runs after apply_damage_events but we read health directly.
pub fn phoenix_system(
    mut commands: Commands,
    data: Res<GameDataRegistry>,
    mut flash_events: EventWriter<crate::core::events::ScreenFlashRequest>,
    mut q: Query<
        (Entity, &mut Health, &AugmentInventory),
        (With<Player>, Without<Replicated>, Without<PhoenixUsed>),
    >,
) {
    for (entity, mut health, inventory) in &mut q {
        if health.current > 0.0 {
            continue;
        }
        let stacks = inventory.stacks(AugmentId::Phoenix);
        if stacks == 0 {
            continue;
        }
        let Some(profile) = tuning::phoenix_profile(&data, stacks) else {
            continue;
        };
        health.current = health.max * profile.revive_fraction;
        commands.entity(entity).insert(PhoenixUsed);
        flash_events.send(crate::core::events::ScreenFlashRequest {
            color: Color::srgba(1.0, 0.85, 0.3, 0.9),
            duration_s: 0.4,
        });
    }
}

fn projectile_reflect_direction(projectile_velocity: Vec2, slash_direction: Vec2) -> Vec2 {
    let slash = slash_direction.normalize_or_zero();
    let away = (-projectile_velocity).normalize_or_zero();
    let blended = slash * 0.75 + away * 0.25;
    if blended.length_squared() > 0.0 {
        blended.normalize()
    } else if slash.length_squared() > 0.0 {
        slash
    } else if away.length_squared() > 0.0 {
        away
    } else {
        Vec2::X
    }
}
