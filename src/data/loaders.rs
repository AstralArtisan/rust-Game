use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use bevy::prelude::*;

use crate::data::definitions::*;
use crate::data::registry::GameDataRegistry;

pub fn load_all_configs(mut commands: Commands) {
    let registry = match try_load_all() {
        Ok(r) => r,
        Err(err) => {
            warn!("Failed to load configs from assets/configs/*.ron, using defaults: {err:?}");
            default_registry()
        }
    };
    commands.insert_resource(registry);
}

fn try_load_all() -> Result<GameDataRegistry> {
    Ok(GameDataRegistry {
        player: load_ron("assets/configs/player.ron")?,
        enemies: load_ron("assets/configs/enemies.ron")?,
        bosses: load_ron("assets/configs/boss.ron")?,
        rewards: load_ron("assets/configs/rewards.ron")?,
        runes: load_ron("assets/configs/runes.ron")?,
        curses: load_ron("assets/configs/curses.ron")?,
        rooms: load_ron("assets/configs/rooms.ron")?,
        balance: load_ron("assets/configs/game_balance.ron")?,
        augments: load_ron("assets/configs/augments.ron")
            .unwrap_or(AugmentsConfig { augments: vec![] }),
        audio: load_ron("assets/configs/audio.ron").unwrap_or_default(),
        effects: load_ron("assets/configs/effects.ron").unwrap_or_default(),
    })
}

pub fn load_ron<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let value =
        ron::from_str::<T>(&content).with_context(|| format!("parse ron {}", path.display()))?;
    Ok(value)
}

fn default_registry() -> GameDataRegistry {
    GameDataRegistry {
        player: PlayerConfig {
            max_hp: 100.0,
            move_speed: 260.0,
            attack_power: 18.0,
            attack_cooldown_s: 0.70,
            ranged_cooldown_s: 0.80,
            dash_cooldown_s: 1.2,
            dash_speed: 680.0,
            dash_duration_s: 0.12,
            invincibility_s: 0.35,
            crit_chance: 0.05,
            energy_max: 100.0,
            melee_charge_gain: 8.0,
            ranged_charge_gain: 4.0,
            kill_charge_gain: 12.0,
            elite_kill_charge_gain: 25.0,
            perfect_dash_charge_gain: 15.0,
            combo_charge_gain: 10.0,
            finisher_charge_cost: 100.0,
            skill1_cooldown_s: 1.1,
            ranged_base_cooldown_s: 0.80,
            ranged_min_cooldown_s: 0.80,
            ranged_ramp_max: 1,
        },
        enemies: EnemiesConfig {
            melee_chaser: EnemyStatsConfig {
                max_hp: 44.0,
                move_speed: 172.0,
                attack_damage: 14.0,
                attack_cooldown_s: 0.82,
                aggro_range: 480.0,
                attack_range: 42.0,
                projectile_speed: 0.0,
            },
            ranged_shooter: EnemyStatsConfig {
                max_hp: 34.0,
                move_speed: 130.0,
                attack_damage: 11.0,
                attack_cooldown_s: 1.00,
                aggro_range: 580.0,
                attack_range: 380.0,
                projectile_speed: 470.0,
            },
            charger: EnemyStatsConfig {
                max_hp: 62.0,
                move_speed: 120.0,
                attack_damage: 21.0,
                attack_cooldown_s: 1.35,
                aggro_range: 560.0,
                attack_range: 350.0,
                projectile_speed: 0.0,
            },
            flanker: EnemyStatsConfig {
                max_hp: 46.0,
                move_speed: 208.0,
                attack_damage: 15.0,
                attack_cooldown_s: 0.82,
                aggro_range: 700.0,
                attack_range: 56.0,
                projectile_speed: 0.0,
            },
            sniper: EnemyStatsConfig {
                max_hp: 44.0,
                move_speed: 108.0,
                attack_damage: 20.0,
                attack_cooldown_s: 1.45,
                aggro_range: 860.0,
                attack_range: 620.0,
                projectile_speed: 840.0,
            },
            support_caster: EnemyStatsConfig {
                max_hp: 54.0,
                move_speed: 126.0,
                attack_damage: 0.0,
                attack_cooldown_s: 1.90,
                aggro_range: 700.0,
                attack_range: 260.0,
                projectile_speed: 0.0,
            },
            bomber: EnemyStatsConfig {
                max_hp: 30.0,
                move_speed: 185.0,
                attack_damage: 28.0,
                attack_cooldown_s: 1.0,
                aggro_range: 560.0,
                attack_range: 55.0,
                projectile_speed: 0.0,
            },
            shielder: EnemyStatsConfig {
                max_hp: 72.0,
                move_speed: 80.0,
                attack_damage: 12.0,
                attack_cooldown_s: 1.2,
                aggro_range: 540.0,
                attack_range: 40.0,
                projectile_speed: 0.0,
            },
            summoner: EnemyStatsConfig {
                max_hp: 28.0,
                move_speed: 95.0,
                attack_damage: 8.0,
                attack_cooldown_s: 4.0,
                aggro_range: 760.0,
                attack_range: 500.0,
                projectile_speed: 320.0,
            },
        },
        bosses: BossesConfig {
            floor_1: BossFloorConfig {
                max_hp: 245.0,
                move_speed: 115.0,
                contact_damage: 13.0,
                phase_thresholds: vec![0.60, 0.30],
                projectile_speed: 430.0,
            },
            floor_2: BossFloorConfig {
                max_hp: 305.0,
                move_speed: 122.0,
                contact_damage: 15.0,
                phase_thresholds: vec![0.68, 0.34],
                projectile_speed: 470.0,
            },
            floor_3: BossFloorConfig {
                max_hp: 372.0,
                move_speed: 128.0,
                contact_damage: 17.0,
                phase_thresholds: vec![0.70, 0.35],
                projectile_speed: 505.0,
            },
            floor_4: BossFloorConfig {
                max_hp: 680.0,
                move_speed: 118.0,
                contact_damage: 19.0,
                phase_thresholds: vec![0.72, 0.38],
                projectile_speed: 540.0,
            },
        },
        rewards: RewardsConfig {
            rewards: vec![],
            scaling: RewardScalingConfig::default_config(),
        },
        runes: RunesConfig { runes: vec![] },
        curses: CursesConfig { curses: vec![] },
        rooms: RoomGenConfig {
            room_sequence: vec![],
        },
        balance: GameBalanceConfig {
            difficulty_per_floor: 0.16,
            enemy_count_normal_room: 4,
            reward_rooms_give_choice: true,
            boss_room_gives_victory: false,
            total_floors: 4,
            floor_rooms: 7,
            enemy_types: vec![],
            elite_chance: 0.18,
            elite_hp_mult: 2.0,
            elite_damage_mult: 1.55,
            elite_gold_bonus: 5,
        },
        augments: AugmentsConfig { augments: vec![] },
        audio: AudioConfig::default(),
        effects: EffectsConfig::default(),
    }
}
