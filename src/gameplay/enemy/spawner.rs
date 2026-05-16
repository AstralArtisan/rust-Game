use bevy::prelude::*;

use crate::data::registry::GameDataRegistry;
use crate::gameplay::enemy::components::EnemyType;
use crate::utils::rng::GameRng;

pub fn get_spawn_points_for_room() -> Vec<Vec2> {
    vec![
        Vec2::new(200.0, 120.0),
        Vec2::new(260.0, -90.0),
        Vec2::new(-60.0, 160.0),
        Vec2::new(30.0, -140.0),
        Vec2::new(120.0, 30.0),
        Vec2::new(-150.0, -60.0),
    ]
}

/// Built-in per-floor unlock table (spec §7.1). Index 0 = floor 1; each inner
/// list = enemies NEWLY unlocked at that floor. Used only when
/// `game_balance.ron` does not provide `enemy_pools_by_floor`.
fn builtin_enemy_pools() -> Vec<Vec<EnemyType>> {
    vec![
        vec![
            EnemyType::MeleeChaser,
            EnemyType::Lobber,
            EnemyType::Charger,
        ],
        vec![
            EnemyType::RangedShooter,
            EnemyType::Flanker,
            EnemyType::Bomber,
        ],
        vec![EnemyType::Sniper, EnemyType::Shielder],
        vec![EnemyType::SupportCaster, EnemyType::Summoner],
    ]
}

/// Enemy pool for `floor` = cumulative union of the per-floor unlock lists for
/// floors `1..=floor`. Always floor-gated; there is no floor-agnostic override.
pub fn choose_enemy_types(data: &GameDataRegistry, floor: u32) -> Vec<EnemyType> {
    let builtin;
    let pools: &Vec<Vec<EnemyType>> = if data.balance.enemy_pools_by_floor.is_empty() {
        builtin = builtin_enemy_pools();
        &builtin
    } else {
        &data.balance.enemy_pools_by_floor
    };
    if pools.is_empty() {
        return vec![EnemyType::MeleeChaser];
    }
    let last = ((floor.max(1) as usize) - 1).min(pools.len() - 1);
    let mut pool: Vec<EnemyType> = Vec::new();
    for tier in &pools[..=last] {
        for &enemy_type in tier {
            if !pool.contains(&enemy_type) {
                pool.push(enemy_type);
            }
        }
    }
    if pool.is_empty() {
        pool.push(EnemyType::MeleeChaser);
    }
    pool
}

pub fn frontline_enemy_types(pool: &[EnemyType]) -> Vec<EnemyType> {
    let filtered = pool
        .iter()
        .copied()
        .filter(|enemy_type| {
            matches!(
                enemy_type,
                EnemyType::MeleeChaser
                    | EnemyType::Charger
                    | EnemyType::Flanker
                    | EnemyType::Bomber
                    | EnemyType::Shielder
            )
        })
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        pool.to_vec()
    } else {
        filtered
    }
}

pub fn backline_enemy_types(pool: &[EnemyType]) -> Vec<EnemyType> {
    let filtered = pool
        .iter()
        .copied()
        .filter(|enemy_type| {
            matches!(
                enemy_type,
                EnemyType::RangedShooter
                    | EnemyType::Lobber
                    | EnemyType::Sniper
                    | EnemyType::SupportCaster
                    | EnemyType::Summoner
            )
        })
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        pool.to_vec()
    } else {
        filtered
    }
}

pub fn pick_enemy_type(rng: &mut GameRng, pool: &[EnemyType]) -> EnemyType {
    if pool.is_empty() {
        return EnemyType::MeleeChaser;
    }
    let i = (rng.gen_range_f32(0.0, pool.len() as f32) as usize).min(pool.len() - 1);
    pool[i]
}
