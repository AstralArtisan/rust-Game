use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::coop::net::is_coop_authority;
use crate::coop::runtime::is_coop_simulation_active;
use crate::core::assets::GameAssets;
use crate::core::events::{DeathEvent, SfxEvent, SfxKind};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::combat::components::Team;
use crate::gameplay::enemy::components::{BossSubCore, Elite, EnemyKind, EnemyType};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::player::components::{Gold, Player};
use crate::gameplay::progression::experience::XpGainEvent;
use crate::gameplay::progression::floor::FloorNumber;
use crate::states::AppState;
use crate::utils::entity::safe_despawn_recursive;
use crate::utils::rng::GameRng;

pub struct DropPlugin;

impl Plugin for DropPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_drops_on_death.before(crate::gameplay::enemy::systems::enemy_death_system),
                drop_physics,
                drop_magnet,
                drop_collect,
                drop_expire,
            )
                .run_if(
                    in_state(AppState::InGame).or_else(
                        in_state(AppState::CoopGame)
                            .and_then(is_coop_authority)
                            .and_then(is_coop_simulation_active),
                    ),
                ),
        );
    }
}

// --- COMPONENTS ---

#[derive(Component)]
pub struct DroppedItem {
    pub kind: DropKind,
    pub value: u32,
    pub lifetime: Timer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropKind {
    Gold,
    Xp,
}

#[derive(Component)]
pub struct DropVelocity(pub Vec2);

#[derive(Component)]
pub struct DropBob(pub f32);

// --- SYSTEMS ---

pub fn spawn_drops_on_death(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut death_events: EventReader<DeathEvent>,
    mut rng: ResMut<GameRng>,
    data: Res<GameDataRegistry>,
    floor: Option<Res<FloorNumber>>,
    enemy_q: Query<(&GlobalTransform, &EnemyKind, Option<&Elite>), Without<Replicated>>,
    sub_core_q: Query<(), With<BossSubCore>>,
    player_q: Query<Option<&AugmentInventory>, (With<Player>, Without<Replicated>)>,
) {
    let floor_number = floor.as_deref().map(|f| f.0).unwrap_or(1);

    for ev in death_events.read() {
        if ev.team != Team::Enemy {
            continue;
        }
        let Ok((tf, enemy_kind, elite)) = enemy_q.get(ev.entity) else {
            continue;
        };
        if sub_core_q.get(ev.entity).is_ok() {
            continue;
        }

        let pos = tf.translation().truncate();
        let kind = enemy_kind.0;
        let is_elite = elite.is_some();

        // Gold calculation (same as old enemy_death_system)
        let base_gold: u32 = match kind {
            EnemyType::Boss => match floor_number {
                1 => 30,
                2 => 45,
                3 => 58,
                _ => 70,
            },
            _ => match floor_number {
                1 => 8,
                2 => 10,
                3 => 13,
                _ => 16,
            },
        };
        let reward_gold = base_gold
            + if is_elite {
                data.balance.elite_gold_bonus
            } else {
                0
            };

        // XP calculation (raw, XpBonus applied in experience.rs)
        let xp_amount: u32 = match kind {
            EnemyType::Boss => 100 + (floor_number.saturating_sub(1) * 30).min(100),
            _ if is_elite => 25 + (floor_number.saturating_sub(1) * 5).min(15),
            _ => 8 + (floor_number.saturating_sub(1) * 2).min(7),
        };

        // Spawn gold drops per player (GoldBonus applied here)
        for inventory in &player_q {
            let gold_mult: f32 = match inventory
                .map(|inv| inv.stacks(AugmentId::GoldBonus))
                .unwrap_or(0)
            {
                2 => 1.50,
                1 => 1.25,
                _ => 1.0,
            };
            let final_gold = (reward_gold as f32 * gold_mult) as u32;
            if final_gold > 0 {
                spawn_drop(
                    &mut commands,
                    &assets,
                    &mut rng,
                    pos,
                    DropKind::Gold,
                    final_gold,
                );
            }
        }

        // Spawn single XP drop (XpBonus handled downstream)
        if xp_amount > 0 {
            spawn_drop(
                &mut commands,
                &assets,
                &mut rng,
                pos,
                DropKind::Xp,
                xp_amount,
            );
        }
    }
}

fn spawn_drop(
    commands: &mut Commands,
    assets: &GameAssets,
    rng: &mut GameRng,
    pos: Vec2,
    kind: DropKind,
    value: u32,
) {
    let angle = rng.gen_range_f32(0.0, std::f32::consts::TAU);
    let speed = rng.gen_range_f32(120.0, 180.0);
    let vel = Vec2::new(angle.cos(), angle.sin()) * speed;

    let (color, size, name) = match kind {
        DropKind::Gold => (Color::srgb(0.95, 0.85, 0.25), Vec2::splat(8.0), "GoldDrop"),
        DropKind::Xp => (Color::srgb(0.35, 0.85, 0.95), Vec2::splat(6.0), "XpDrop"),
    };

    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(45.0)),
            sprite: Sprite {
                color,
                custom_size: Some(size),
                ..default()
            },
            ..default()
        },
        DroppedItem {
            kind,
            value,
            lifetime: Timer::from_seconds(15.0, TimerMode::Once),
        },
        DropVelocity(vel),
        DropBob(rng.gen_range_f32(0.0, std::f32::consts::TAU)),
        InGameEntity,
        Name::new(name),
    ));
}

pub fn drop_physics(
    time: Res<Time>,
    mut q: Query<(&mut DropVelocity, &mut DropBob, &mut Transform)>,
) {
    let dt = time.delta_seconds();
    for (mut vel, mut bob, mut tf) in &mut q {
        vel.0 *= (-6.0 * dt).exp();
        bob.0 += dt * 4.0;
        tf.translation.x += vel.0.x * dt;
        tf.translation.y += vel.0.y * dt;
        // Bob effect on z to avoid interfering with Y position
        tf.translation.z = 45.0 + bob.0.sin() * 2.0;
    }
}

pub fn drop_magnet(
    time: Res<Time>,
    player_q: Query<
        (&GlobalTransform, Option<&AugmentInventory>),
        (With<Player>, Without<Replicated>),
    >,
    mut drop_q: Query<(&DroppedItem, &mut DropVelocity, &GlobalTransform), Without<Player>>,
) {
    let dt = time.delta_seconds();
    for (_, mut vel, drop_tf) in &mut drop_q {
        let drop_pos = drop_tf.translation().truncate();
        let mut closest_dist = f32::MAX;
        let mut closest_dir = Vec2::ZERO;
        let mut closest_range = 140.0f32;

        for (player_tf, inventory) in &player_q {
            let player_pos = player_tf.translation().truncate();
            let dist = drop_pos.distance(player_pos);
            let pickup_mult: f32 = match inventory
                .map(|inv| inv.stacks(AugmentId::PickupRange))
                .unwrap_or(0)
            {
                2 => 2.0,
                1 => 1.6,
                _ => 1.0,
            };
            let range = 140.0 * pickup_mult;
            if dist < closest_dist {
                closest_dist = dist;
                closest_dir = (player_pos - drop_pos).normalize_or_zero();
                closest_range = range;
            }
        }

        if closest_dist < closest_range {
            vel.0 += closest_dir * 600.0 * dt;
        }
    }
}

pub fn drop_collect(
    mut commands: Commands,
    mut sfx_events: EventWriter<SfxEvent>,
    mut xp_events: EventWriter<XpGainEvent>,
    mut player_q: Query<(&GlobalTransform, &mut Gold), (With<Player>, Without<Replicated>)>,
    drop_q: Query<(Entity, &DroppedItem, &GlobalTransform), Without<Player>>,
) {
    for (drop_entity, item, drop_tf) in &drop_q {
        let drop_pos = drop_tf.translation().truncate();

        // Find closest player
        let mut closest = None;
        let mut closest_dist = f32::MAX;
        for (player_tf, _) in &player_q {
            let dist = drop_pos.distance(player_tf.translation().truncate());
            if dist < closest_dist {
                closest_dist = dist;
                closest = Some(player_tf.translation().truncate());
            }
        }

        if closest_dist >= 28.0 {
            continue;
        }

        match item.kind {
            DropKind::Gold => {
                for (_, mut gold) in &mut player_q {
                    gold.0 = gold.0.saturating_add(item.value);
                }
            }
            DropKind::Xp => {
                xp_events.send(XpGainEvent { amount: item.value });
            }
        }
        sfx_events.send(SfxEvent {
            kind: SfxKind::RewardPickup,
        });
        safe_despawn_recursive(&mut commands, drop_entity);
    }
}

pub fn drop_expire(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut DroppedItem)>,
) {
    for (entity, mut item) in &mut q {
        item.lifetime.tick(time.delta());
        if item.lifetime.finished() {
            safe_despawn_recursive(&mut commands, entity);
        }
    }
}
