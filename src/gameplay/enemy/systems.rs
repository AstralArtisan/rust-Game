use bevy::prelude::*;

use crate::coop::components::CoopPlayer;
use crate::core::events::{DeathEvent, RoomClearedEvent};
use crate::data::definitions::EnemyStatsConfig;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::combat::components::{Hitbox, Hurtbox, Knockback, Lifetime, Projectile, Team};
use crate::gameplay::combat::projectiles;
use crate::gameplay::effects::flash::Flash;
use crate::gameplay::effects::particles;
use crate::gameplay::enemy::{ai, boss, spawner};
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomType};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::puzzle::{reset_active_puzzle, spawn_puzzle_for_room, ActivePuzzle, PuzzleEntity};
use crate::gameplay::player::components::{Gold, Health, RewardModifiers};
use crate::gameplay::player::components::{Health as PlayerHealth, Player};
use crate::gameplay::progression::difficulty::{
    get_floor_difficulty_multiplier, get_floor_enemy_count,
};
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::shop::ShopKiosk;
use crate::states::{AppState, RoomState};
use crate::utils::math::direction_to;
use crate::utils::rng::GameRng;

use super::components::*;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct EnemyVelocity(pub Vec2);

pub struct EnemySystemsPlugin;

impl Plugin for EnemySystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameRng>()
            .insert_resource(EnemySpawnCount { current: 0 })
            .insert_resource(SpawnedForRoom::default())
            .insert_resource(ClearGrace::default())
            .add_systems(
                Update,
                (
                    room_entry_spawner,
                    ai::update_enemy_ai,
                    enemy_attack_system,
                    boss::boss_phase_controller,
                    boss::boss_attack_patterns,
                    enemy_death_system,
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub(crate) struct SpawnedForRoom(pub Option<u32>);

#[derive(Resource, Debug)]
pub(crate) struct ClearGrace {
    pub(crate) last_room: Option<u32>,
    pub(crate) timer: Timer,
}

impl Default for ClearGrace {
    fn default() -> Self {
        Self {
            last_room: None,
            timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct EnemySpawnCount {
    pub current: u32,
}

pub fn room_entry_spawner(
    mut commands: Commands,
    mut spawned: ResMut<SpawnedForRoom>,
    data: Res<GameDataRegistry>,
    assets: Res<crate::core::assets::GameAssets>,
    layout: Res<FloorLayout>,
    current_room: Res<CurrentRoom>,
    mut room_state: ResMut<RoomState>,
    enemies_q: Query<Entity, With<Enemy>>,
    projectiles_q: Query<Entity, With<Projectile>>,
    hitboxes_q: Query<Entity, With<Hitbox>>,
    shop_kiosk_q: Query<Entity, With<ShopKiosk>>,
    puzzle_entities_q: Query<Entity, With<PuzzleEntity>>,
    mut spawn_count: ResMut<EnemySpawnCount>,
    mut active_puzzle: ResMut<ActivePuzzle>,
    mut rng: ResMut<GameRng>,
    floor: Option<Res<FloorNumber>>,
) {
    if spawned.0 == Some(current_room.0.0) {
        return;
    }
    spawned.0 = Some(current_room.0.0);

    for entity in &enemies_q {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &projectiles_q {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &hitboxes_q {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &shop_kiosk_q {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &puzzle_entities_q {
        commands.entity(entity).despawn_recursive();
    }
    reset_active_puzzle(&mut active_puzzle);

    let room = layout.room(current_room.0).unwrap();
    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
    let floor_multiplier = get_floor_difficulty_multiplier(&data, floor_number);
    let base_enemy_count = get_floor_enemy_count(&data, floor_number);

    match room.room_type {
        RoomType::Start | RoomType::Reward | RoomType::Shop => {
            *room_state = RoomState::Idle;
        }
        RoomType::Normal => {
            *room_state = RoomState::Locked;
            if spawn_count.current == 0 {
                spawn_count.current = base_enemy_count;
            }
            spawn_room_enemies(
                &mut commands,
                &assets,
                &data,
                spawn_count.current,
                floor_multiplier,
            );
        }
        RoomType::Boss => {
            *room_state = RoomState::BossFight;
            spawn_boss(&mut commands, &assets, &data, floor_multiplier);
        }
        RoomType::Puzzle => {
            *room_state = RoomState::Locked;
            spawn_puzzle_for_room(
                &mut commands,
                &assets,
                &mut rng,
                &mut active_puzzle,
                current_room.0,
            );
        }
    }
}

pub fn spawn_room_enemies(
    commands: &mut Commands,
    assets: &crate::core::assets::GameAssets,
    data: &GameDataRegistry,
    enemy_count: u32,
    floor_multiplier: f32,
) {
    let points = spawner::get_spawn_points_for_room();
    let pool = spawner::choose_enemy_types(data);
    let count = enemy_count as usize;
    let mut rng = GameRng::default();
    let spawn_n = count.min(points.len());

    let elite_idx = if spawn_n > 0 && rng.gen_range_f32(0.0, 1.0) < data.balance.elite_chance {
        Some((rng.gen_range_f32(0.0, spawn_n as f32) as usize).min(spawn_n - 1))
    } else {
        None
    };

    for i in 0..spawn_n {
        let enemy_type = spawner::pick_enemy_type(&mut rng, &pool);
        let is_elite = elite_idx == Some(i);
        spawn_enemy(
            commands,
            assets,
            data,
            enemy_type,
            points[i],
            floor_multiplier,
            is_elite,
        );
    }
}

pub fn spawn_enemy(
    commands: &mut Commands,
    assets: &crate::core::assets::GameAssets,
    data: &GameDataRegistry,
    enemy_type: EnemyType,
    pos: Vec2,
    floor_multiplier: f32,
    is_elite: bool,
) -> Entity {
    let stats_cfg = match enemy_type {
        EnemyType::MeleeChaser => &data.enemies.melee_chaser,
        EnemyType::RangedShooter => &data.enemies.ranged_shooter,
        EnemyType::Charger => &data.enemies.charger,
        EnemyType::Boss => &data.enemies.melee_chaser,
    };
    let mut stats = scaled_enemy_stats(stats_cfg, floor_multiplier);
    if is_elite && enemy_type != EnemyType::Boss {
        stats.max_hp *= data.balance.elite_hp_mult.max(1.0);
        stats.attack_damage *= data.balance.elite_damage_mult.max(1.0);
        stats.move_speed *= 1.12;
    }
    let base_color = match enemy_type {
        EnemyType::MeleeChaser => Color::srgb(0.95, 0.45, 0.45),
        EnemyType::RangedShooter => Color::srgb(0.55, 0.65, 0.95),
        EnemyType::Charger => Color::srgb(0.95, 0.75, 0.25),
        EnemyType::Boss => Color::srgb(0.85, 0.25, 0.95),
    };
    let color = if is_elite && enemy_type != EnemyType::Boss {
        match enemy_type {
            EnemyType::MeleeChaser => Color::srgb(1.0, 0.65, 0.65),
            EnemyType::RangedShooter => Color::srgb(0.75, 0.82, 1.0),
            EnemyType::Charger => Color::srgb(1.0, 0.88, 0.45),
            EnemyType::Boss => base_color,
        }
    } else {
        base_color
    };

    let mut entity = commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(45.0)),
            sprite: Sprite {
                color,
                custom_size: Some(Vec2::splat(if enemy_type == EnemyType::Boss {
                    56.0
                } else if is_elite {
                    34.0
                } else {
                    28.0
                })),
                ..default()
            },
            ..default()
        },
        Enemy,
        EnemyKind(enemy_type),
        TeamMarker(Team::Enemy),
        Health {
            current: stats.max_hp,
            max: stats.max_hp,
        },
        stats,
        EnemyAttackCooldown {
            timer: Timer::from_seconds(stats.attack_cooldown_s, TimerMode::Once),
        },
        EnemyVelocity::default(),
        Hurtbox {
            team: Team::Enemy,
            size: Vec2::splat(if enemy_type == EnemyType::Boss {
                52.0
            } else if is_elite {
                32.0
            } else {
                26.0
            }),
        },
        Flash::new(0.0),
        Knockback(Vec2::ZERO),
        InGameEntity,
        Name::new("Enemy"),
    ));

    if is_elite && enemy_type != EnemyType::Boss {
        entity.insert(Elite);
    }

    if enemy_type == EnemyType::Charger {
        entity.insert(ChargerState {
            phase: ChargerPhase::Idle,
            timer: Timer::from_seconds(0.1, TimerMode::Once),
            dir: Vec2::X,
        });
    }

    entity.id()
}

pub fn spawn_boss(
    commands: &mut Commands,
    assets: &crate::core::assets::GameAssets,
    data: &GameDataRegistry,
    floor_multiplier: f32,
) -> Entity {
    let stats = scaled_boss_stats(data, floor_multiplier);
    let id = commands
        .spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(Vec3::new(220.0, 0.0, 45.0)),
                sprite: Sprite {
                    color: Color::srgb(0.85, 0.25, 0.95),
                    custom_size: Some(Vec2::splat(64.0)),
                    ..default()
                },
                ..default()
            },
            Enemy,
            TeamMarker(Team::Enemy),
            Health {
                current: stats.max_hp,
                max: stats.max_hp,
            },
            stats,
            EnemyVelocity::default(),
            Hurtbox {
                team: Team::Enemy,
                size: Vec2::splat(60.0),
            },
            Flash::new(0.0),
            Knockback(Vec2::ZERO),
            InGameEntity,
            Name::new("Boss"),
        ))
        .id();
    let (kind, phase, timer) = boss::spawn_boss_bundle(data);
    commands.entity(id).insert((kind, phase, timer));
    id
}

pub fn enemy_attack_system(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<crate::core::assets::GameAssets>,
    player_q: Query<&GlobalTransform, Or<(With<Player>, With<CoopPlayer>)>>,
    mut enemies: Query<(
        &EnemyKind,
        &EnemyStats,
        &GlobalTransform,
        &mut EnemyAttackCooldown,
    )>,
) {
    let player_positions: Vec<Vec2> = player_q
        .iter()
        .map(|tf| tf.translation().truncate())
        .collect();
    if player_positions.is_empty() {
        return;
    }

    for (kind, stats, tf, mut cd) in &mut enemies {
        cd.timer.tick(time.delta());
        if !cd.timer.finished() {
            continue;
        }
        let pos = tf.translation().truncate();
        let (player_pos, dist) = player_positions
            .iter()
            .map(|p| (*p, pos.distance(*p)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        if dist > stats.aggro_range {
            continue;
        }

        match kind.0 {
            EnemyType::MeleeChaser | EnemyType::Charger => {
                if dist <= stats.attack_range {
                    cd.timer.reset();
                    spawn_enemy_melee_hitbox(
                        &mut commands,
                        &assets,
                        pos,
                        direction_to(pos, player_pos),
                        stats.attack_damage,
                    );
                }
            }
            EnemyType::RangedShooter => {
                if dist <= stats.attack_range {
                    cd.timer.reset();
                    let dir = direction_to(pos, player_pos);
                    projectiles::spawn_projectile(
                        &mut commands,
                        &assets,
                        Team::Enemy,
                        pos + dir * 18.0,
                        dir * stats.projectile_speed,
                        stats.attack_damage,
                    );
                }
            }
            EnemyType::Boss => {}
        }
    }
}

fn spawn_enemy_melee_hitbox(
    commands: &mut Commands,
    assets: &crate::core::assets::GameAssets,
    pos: Vec2,
    dir: Vec2,
    damage: f32,
) {
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation((pos + dir * 24.0).extend(55.0)),
            sprite: Sprite {
                color: Color::srgba(1.0, 0.3, 0.25, 0.28),
                custom_size: Some(Vec2::new(40.0, 22.0)),
                ..default()
            },
            ..default()
        },
        Hitbox {
            owner: None,
            team: Team::Enemy,
            size: Vec2::new(40.0, 22.0),
            damage,
            knockback: 300.0,
            can_crit: false,
            crit_chance: 0.0,
            crit_multiplier: 1.0,
        },
        Lifetime(Timer::from_seconds(0.10, TimerMode::Once)),
        InGameEntity,
        Name::new("EnemyHitbox"),
    ));
}

pub fn enemy_death_system(
    mut commands: Commands,
    mut death_events: EventReader<DeathEvent>,
    mut room_cleared: EventWriter<RoomClearedEvent>,
    time: Res<Time>,
    assets: Res<crate::core::assets::GameAssets>,
    layout: Res<FloorLayout>,
    current_room: Res<CurrentRoom>,
    mut room_state: ResMut<RoomState>,
    mut player_q: Query<(&RewardModifiers, &mut PlayerHealth, &mut Gold), With<Player>>,
    enemy_info_q: Query<(&EnemyKind, Option<&Elite>)>,
    player_tf: Query<&GlobalTransform, With<Player>>,
    enemies_left: Query<Entity, With<Enemy>>,
    mut grace: ResMut<ClearGrace>,
    mut spawn_count: ResMut<EnemySpawnCount>,
    data: Res<GameDataRegistry>,
    floor: Option<Res<FloorNumber>>,
) {
    for ev in death_events.read() {
        if ev.team != Team::Enemy {
            continue;
        }

        let (kind, is_elite) = enemy_info_q
            .get(ev.entity)
            .ok()
            .map(|(k, e)| (Some(k.0), e.is_some()))
            .unwrap_or((None, false));
        let floor_bonus = floor.as_deref().map(|f| f.0.saturating_sub(1)).unwrap_or(0);
        let base_gold = match kind {
            Some(EnemyType::Boss) => 45,
            _ => 10,
        } + floor_bonus * 2;
        let reward_gold = base_gold + if is_elite { data.balance.elite_gold_bonus } else { 0 };

        if let Ok((mods, mut hp, mut gold)) = player_q.get_single_mut() {
            gold.0 = gold.0.saturating_add(reward_gold);
            if mods.lifesteal_on_kill > 0.0 {
                let previous = hp.current;
                hp.current = (hp.current + mods.lifesteal_on_kill).min(hp.max);
                if hp.current > previous {
                    if let Ok(player_tf) = player_tf.get_single() {
                        particles::spawn_hit_particles(
                            &mut commands,
                            &assets,
                            player_tf.translation().truncate(),
                            Color::srgba(0.42, 1.0, 0.52, 0.88),
                        );
                    }
                }
            }
        }

        commands.entity(ev.entity).despawn_recursive();
    }

    if matches!(*room_state, RoomState::Locked | RoomState::BossFight) {
        if grace.last_room != Some(current_room.0.0) {
            grace.last_room = Some(current_room.0.0);
            grace.timer = Timer::from_seconds(0.20, TimerMode::Once);
            grace.timer.reset();
        }
        if !grace.timer.finished() {
            grace.timer.tick(time.delta());
            return;
        }

        let any_enemy_left = enemies_left.iter().next().is_some();
        if !any_enemy_left {
            *room_state = RoomState::Cleared;
            room_cleared.send(RoomClearedEvent {
                room: current_room.0,
            });
            let room = layout.room(current_room.0).unwrap();
            if room.room_type == RoomType::Normal {
                let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
                let minimum = get_floor_enemy_count(&data, floor_number)
                    .saturating_sub(1)
                    .max(2);
                let next = spawn_count.current.saturating_sub(1);
                spawn_count.current = next.max(minimum);
            }
        }
    }
}

fn scaled_enemy_stats(stats_cfg: &EnemyStatsConfig, floor_multiplier: f32) -> EnemyStats {
    let scaling = (floor_multiplier - 1.0).max(0.0);
    EnemyStats {
        max_hp: stats_cfg.max_hp * floor_multiplier,
        move_speed: stats_cfg.move_speed * (1.0 + scaling * 0.20),
        attack_damage: stats_cfg.attack_damage * (1.0 + scaling * 0.75),
        attack_cooldown_s: (stats_cfg.attack_cooldown_s / (1.0 + scaling * 0.18)).max(0.45),
        aggro_range: stats_cfg.aggro_range,
        attack_range: stats_cfg.attack_range,
        projectile_speed: stats_cfg.projectile_speed * (1.0 + scaling * 0.15),
    }
}

fn scaled_boss_stats(data: &GameDataRegistry, floor_multiplier: f32) -> EnemyStats {
    let scaling = (floor_multiplier - 1.0).max(0.0);
    EnemyStats {
        max_hp: data.boss.max_hp * (1.0 + scaling * 1.1),
        move_speed: data.boss.move_speed * (1.0 + scaling * 0.15),
        attack_damage: data.boss.contact_damage * (1.0 + scaling * 0.70),
        attack_cooldown_s: 1.0 / (1.0 + scaling * 0.15),
        aggro_range: 900.0,
        attack_range: 42.0,
        projectile_speed: data.boss.projectile_speed * (1.0 + scaling * 0.20),
    }
}
