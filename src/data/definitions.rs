use serde::{Deserialize, Serialize};

use crate::gameplay::augment::data::{AugmentCategory, AugmentId, AugmentRarity};
use crate::gameplay::curse::CurseId;
use crate::gameplay::enemy::components::EnemyType;
use crate::gameplay::map::room::RoomType;
use crate::gameplay::rewards::data::RewardType;
use crate::gameplay::rune::data::{RuneId, RuneSlot, RuneTier};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    pub max_hp: f32,
    pub move_speed: f32,
    pub attack_power: f32,
    pub attack_cooldown_s: f32,
    pub ranged_cooldown_s: f32,
    pub dash_cooldown_s: f32,
    pub dash_speed: f32,
    pub dash_duration_s: f32,
    pub invincibility_s: f32,
    pub crit_chance: f32,
    pub energy_max: f32,
    pub melee_charge_gain: f32,
    pub ranged_charge_gain: f32,
    pub kill_charge_gain: f32,
    pub elite_kill_charge_gain: f32,
    pub perfect_dash_charge_gain: f32,
    pub combo_charge_gain: f32,
    pub finisher_charge_cost: f32,
    pub skill1_cooldown_s: f32,
    pub ranged_base_cooldown_s: f32,
    pub ranged_min_cooldown_s: f32,
    pub ranged_ramp_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyStatsConfig {
    pub max_hp: f32,
    pub move_speed: f32,
    pub attack_damage: f32,
    pub attack_cooldown_s: f32,
    pub aggro_range: f32,
    pub attack_range: f32,
    pub projectile_speed: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemiesConfig {
    pub melee_chaser: EnemyStatsConfig,
    pub ranged_shooter: EnemyStatsConfig,
    pub charger: EnemyStatsConfig,
    pub flanker: EnemyStatsConfig,
    pub sniper: EnemyStatsConfig,
    pub support_caster: EnemyStatsConfig,
    pub bomber: EnemyStatsConfig,
    pub shielder: EnemyStatsConfig,
    pub summoner: EnemyStatsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossFloorConfig {
    pub max_hp: f32,
    pub move_speed: f32,
    pub contact_damage: f32,
    pub phase_thresholds: Vec<f32>,
    pub projectile_speed: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossesConfig {
    pub floor_1: BossFloorConfig,
    pub floor_2: BossFloorConfig,
    pub floor_3: BossFloorConfig,
    pub floor_4: BossFloorConfig,
}

impl BossesConfig {
    pub fn for_floor(&self, floor: u32) -> &BossFloorConfig {
        match floor {
            0 | 1 => &self.floor_1,
            2 => &self.floor_2,
            3 => &self.floor_3,
            _ => &self.floor_4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardConfig {
    pub reward: RewardType,
    pub title: String,
    pub description: String,
    pub value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardsConfig {
    pub rewards: Vec<RewardConfig>,
    #[serde(default = "RewardScalingConfig::default_config")]
    pub scaling: RewardScalingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorGains {
    pub floor_1: f32,
    pub floor_2: f32,
    pub floor_3: f32,
    pub floor_4: f32,
}

impl FloorGains {
    pub fn get(&self, floor: u32) -> f32 {
        match floor {
            0 | 1 => self.floor_1,
            2 => self.floor_2,
            3 => self.floor_3,
            _ => self.floor_4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardScalingConfig {
    pub attack_speed_s: FloorGains,
    pub attack_power: FloorGains,
    pub max_health: FloorGains,
    pub dash_cooldown_s: FloorGains,
    pub lifesteal: FloorGains,
    pub crit_chance: FloorGains,
    pub move_speed: FloorGains,
    pub heal_base: FloorGains,
    pub heal_hp_fraction: f32,
}

impl RewardScalingConfig {
    pub fn default_config() -> Self {
        Self {
            attack_speed_s: FloorGains { floor_1: 0.04, floor_2: 0.06, floor_3: 0.07, floor_4: 0.08 },
            attack_power: FloorGains { floor_1: 4.0, floor_2: 5.0, floor_3: 6.0, floor_4: 7.0 },
            max_health: FloorGains { floor_1: 20.0, floor_2: 24.0, floor_3: 28.0, floor_4: 32.0 },
            dash_cooldown_s: FloorGains { floor_1: 0.08, floor_2: 0.10, floor_3: 0.12, floor_4: 0.14 },
            lifesteal: FloorGains { floor_1: 3.0, floor_2: 4.0, floor_3: 5.0, floor_4: 6.0 },
            crit_chance: FloorGains { floor_1: 0.03, floor_2: 0.04, floor_3: 0.05, floor_4: 0.06 },
            move_speed: FloorGains { floor_1: 18.0, floor_2: 24.0, floor_3: 30.0, floor_4: 36.0 },
            heal_base: FloorGains { floor_1: 24.0, floor_2: 30.0, floor_3: 36.0, floor_4: 42.0 },
            heal_hp_fraction: 0.22,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneConfig {
    pub id: RuneId,
    pub slot: RuneSlot,
    pub tier: RuneTier,
    pub title: String,
    pub description: String,
    pub drawback: String,
    pub shop_cost: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunesConfig {
    pub runes: Vec<RuneConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseConfig {
    pub id: CurseId,
    pub title: String,
    pub description: String,
    pub duration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursesConfig {
    pub curses: Vec<CurseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomGenConfig {
    pub room_sequence: Vec<RoomType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameBalanceConfig {
    pub difficulty_per_floor: f32,
    pub enemy_count_normal_room: u32,
    pub reward_rooms_give_choice: bool,
    pub boss_room_gives_victory: bool,
    pub total_floors: u32,
    pub floor_rooms: u32,
    pub enemy_types: Vec<EnemyType>,
    pub elite_chance: f32,
    pub elite_hp_mult: f32,
    pub elite_damage_mult: f32,
    pub elite_gold_bonus: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentConfig {
    pub id: AugmentId,
    pub category: AugmentCategory,
    pub rarity: AugmentRarity,
    pub title: String,
    pub description: String,
    pub upgraded_description: String,
    pub shop_cost: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentsConfig {
    pub augments: Vec<AugmentConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub bgm_volume: f32,
    pub pitch_variation: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            master_volume: 0.7,
            sfx_volume: 0.8,
            bgm_volume: 0.5,
            pitch_variation: 0.08,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectsConfig {
    pub hit_particle_count: u32,
    pub death_particle_count: u32,
    pub hitstop_duration_s: f32,
    pub hitstop_crit_s: f32,
    pub hitstop_kill_s: f32,
    pub bar_lerp_speed: f32,
    pub screen_flash_duration_s: f32,
    pub death_scale_s: f32,
}

impl Default for EffectsConfig {
    fn default() -> Self {
        Self {
            hit_particle_count: 10,
            death_particle_count: 16,
            hitstop_duration_s: 0.04,
            hitstop_crit_s: 0.06,
            hitstop_kill_s: 0.08,
            bar_lerp_speed: 8.0,
            screen_flash_duration_s: 0.15,
            death_scale_s: 0.25,
        }
    }
}
