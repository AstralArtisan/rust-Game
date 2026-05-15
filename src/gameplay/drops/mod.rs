use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::coop::net::is_coop_authority;
use crate::coop::runtime::is_coop_simulation_active;
use crate::core::assets::GameAssets;
use crate::core::events::{DeathEvent, SfxEvent, SfxKind};
use crate::data::definitions::EconomyConfig;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::augment::tuning;
use crate::gameplay::combat::components::Team;
use crate::gameplay::enemy::components::{BossSubCore, Elite, EnemyKind, EnemyType};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::player::components::{Gold, Player};
use crate::gameplay::progression::experience::XpGainEvent;
use crate::gameplay::progression::floor::FloorNumber;
use crate::states::{AppState, GamePhase};
use crate::utils::entity::safe_despawn_recursive;
use crate::utils::rng::GameRng;

pub struct DropPlugin;

impl Plugin for DropPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_drops_on_death
                    .after(crate::gameplay::combat::damage::apply_damage_events)
                    .before(crate::gameplay::enemy::systems::enemy_death_system),
                drop_physics,
                drop_magnet,
                drop_collect,
                drop_expire,
                update_pickup_texts,
            )
                .run_if(
                    in_state(AppState::InGame)
                        .or_else(
                            in_state(AppState::CoopGame)
                                .and_then(is_coop_authority)
                                .and_then(is_coop_simulation_active),
                        )
                        .and_then(in_state(GamePhase::Playing)),
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

#[derive(Component)]
pub struct PickupText {
    pub timer: Timer,
    pub velocity: Vec2,
}

// --- SYSTEMS ---

pub fn spawn_drops_on_death(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut death_events: EventReader<DeathEvent>,
    mut rng: ResMut<GameRng>,
    data: Res<GameDataRegistry>,
    floor: Option<Res<FloorNumber>>,
    enemy_q: Query<
        (&GlobalTransform, &EnemyKind, Option<&Elite>),
        (
            Without<Replicated>,
            Without<crate::gameplay::enemy::components::SplitSpawn>,
        ),
    >,
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

        let reward_gold = roll_enemy_gold(kind, is_elite, &data.economy, &mut rng);

        // XP calculation (raw, XpBonus applied in experience.rs)
        let xp_amount: u32 = match kind {
            EnemyType::Boss => 120 + (floor_number.saturating_sub(1) * 15).min(45),
            _ if is_elite => 40 + (floor_number.saturating_sub(1) * 5).min(15),
            _ => 10 + (floor_number.saturating_sub(1) * 1).min(3),
        };

        let base_gold_drops: u32 = match kind {
            EnemyType::Boss => 8,
            _ if is_elite => 4,
            _ => 1,
        };
        let base_xp_drops: u32 = match kind {
            EnemyType::Boss => 6,
            _ if is_elite => 3,
            _ => 1,
        };
        let floor_mult: u32 = if floor_number >= 3 { 2 } else { 1 };
        let is_boss_or_elite = matches!(kind, EnemyType::Boss) || is_elite;
        let gold_drop_count = if is_boss_or_elite {
            base_gold_drops * floor_mult
        } else {
            base_gold_drops
        };
        let xp_drop_count = if is_boss_or_elite {
            base_xp_drops * floor_mult
        } else {
            base_xp_drops
        };

        // Spawn gold drops per player (GoldBonus applied here)
        for inventory in &player_q {
            let gold_mult = tuning::gold_bonus_mult(
                inventory
                    .map(|inv| inv.stacks(AugmentId::GoldBonus))
                    .unwrap_or(0),
            );
            let final_gold = (reward_gold as f32 * gold_mult) as u32;
            if final_gold > 0 {
                let gold_per = (final_gold / gold_drop_count).max(1);
                let gold_remainder = final_gold.saturating_sub(gold_per * gold_drop_count);
                for i in 0..gold_drop_count {
                    let value = gold_per + if i == 0 { gold_remainder } else { 0 };
                    if value > 0 {
                        spawn_drop(&mut commands, &assets, &mut rng, pos, DropKind::Gold, value);
                    }
                }
            }
        }

        // Spawn XP drops (XpBonus handled downstream)
        if xp_amount > 0 {
            let xp_per = (xp_amount / xp_drop_count).max(1);
            let xp_remainder = xp_amount.saturating_sub(xp_per * xp_drop_count);
            for i in 0..xp_drop_count {
                let value = xp_per + if i == 0 { xp_remainder } else { 0 };
                if value > 0 {
                    spawn_drop(&mut commands, &assets, &mut rng, pos, DropKind::Xp, value);
                }
            }
        }
    }
}

fn roll_enemy_gold(
    enemy_type: EnemyType,
    is_elite: bool,
    economy: &EconomyConfig,
    rng: &mut GameRng,
) -> u32 {
    let range = if enemy_type == EnemyType::Boss {
        economy.boss_gold
    } else if is_elite {
        economy.elite_gold
    } else {
        economy.normal_gold
    };
    roll_u32_inclusive(range, rng)
}

fn roll_u32_inclusive([min, max]: [u32; 2], rng: &mut GameRng) -> u32 {
    if max <= min {
        return min;
    }
    let upper_exclusive = max.saturating_add(1);
    (rng.gen_range_f32(min as f32, upper_exclusive as f32)
        .floor() as u32)
        .clamp(min, max)
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
        DropKind::Gold => (Color::srgb(1.0, 0.92, 0.20), Vec2::splat(12.0), "GoldDrop"),
        DropKind::Xp => (Color::srgb(0.30, 0.75, 1.0), Vec2::splat(10.0), "XpDrop"),
    };

    let glow_color = match kind {
        DropKind::Gold => Color::srgba(1.0, 0.92, 0.20, 0.25),
        DropKind::Xp => Color::srgba(0.30, 0.75, 1.0, 0.25),
    };

    commands
        .spawn((
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
                lifetime: Timer::from_seconds(8.0, TimerMode::Once),
            },
            DropVelocity(vel),
            DropBob(rng.gen_range_f32(0.0, std::f32::consts::TAU)),
            InGameEntity,
            Name::new(name),
        ))
        .with_children(|parent| {
            parent.spawn(SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
                sprite: Sprite {
                    color: glow_color,
                    custom_size: Some(size * 2.5),
                    ..default()
                },
                ..default()
            });
        });
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
            let pickup_mult = tuning::pickup_range_mult(
                inventory
                    .map(|inv| inv.stacks(AugmentId::PickupRange))
                    .unwrap_or(0),
            );
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
    assets: Res<GameAssets>,
    mut sfx_events: EventWriter<SfxEvent>,
    mut xp_events: EventWriter<XpGainEvent>,
    mut player_q: Query<(&GlobalTransform, &mut Gold), (With<Player>, Without<Replicated>)>,
    drop_q: Query<(Entity, &DroppedItem, &GlobalTransform), Without<Player>>,
) {
    for (drop_entity, item, drop_tf) in &drop_q {
        let drop_pos = drop_tf.translation().truncate();

        // Find closest player
        let mut closest_dist = f32::MAX;
        for (player_tf, _) in &player_q {
            let dist = drop_pos.distance(player_tf.translation().truncate());
            if dist < closest_dist {
                closest_dist = dist;
            }
        }

        if closest_dist >= 36.0 {
            continue;
        }

        let (text_str, text_color) = match item.kind {
            DropKind::Gold => (format!("+{}", item.value), Color::srgb(1.0, 0.92, 0.20)),
            DropKind::Xp => (format!("+{}XP", item.value), Color::srgb(0.30, 0.75, 1.0)),
        };

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

        // Spawn pickup floating text
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    text_str,
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 18.0,
                        color: text_color,
                    },
                ),
                transform: Transform::from_translation(
                    (drop_pos + Vec2::new(0.0, 12.0)).extend(90.0),
                ),
                ..default()
            },
            PickupText {
                timer: Timer::from_seconds(0.6, TimerMode::Once),
                velocity: Vec2::new(0.0, 60.0),
            },
            InGameEntity,
            Name::new("PickupText"),
        ));

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

pub fn update_pickup_texts(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut PickupText, &mut Transform, &mut Text)>,
) {
    for (entity, mut pt, mut tf, mut text) in &mut q {
        pt.timer.tick(time.delta());
        tf.translation += (pt.velocity * time.delta_seconds()).extend(0.0);
        let t = pt.timer.fraction();
        let alpha = (1.0 - t).clamp(0.0, 1.0);
        if let Some(section) = text.sections.get_mut(0) {
            section.style.color.set_alpha(alpha);
        }
        if pt.timer.finished() {
            safe_despawn_recursive(&mut commands, entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase3_gold_economy_rolls_within_configured_ranges() {
        let economy = EconomyConfig::default();
        let mut rng = GameRng::default();
        rng.reseed(17);

        for _ in 0..32 {
            assert!((3..=6).contains(&roll_enemy_gold(
                EnemyType::MeleeChaser,
                false,
                &economy,
                &mut rng
            )));
            assert!((12..=20).contains(&roll_enemy_gold(
                EnemyType::MeleeChaser,
                true,
                &economy,
                &mut rng
            )));
            assert!((30..=50).contains(&roll_enemy_gold(
                EnemyType::Boss,
                false,
                &economy,
                &mut rng
            )));
        }
    }
}
