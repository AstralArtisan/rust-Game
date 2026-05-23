use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::gameplay::augment::data::{AugmentCategory, AugmentId, AugmentRarity};
use crate::gameplay::enemy::components::{EliteAffix, EnemyType};
use crate::gameplay::map::room::RoomType;
use crate::gameplay::player::components::SkillType;
use crate::gameplay::rewards::data::RewardType;

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
pub struct ChargerConfig {
    pub charge_duration_s: f32,
    pub charge_speed_mult: f32,
    pub wall_stun_s: f32,
    pub cooldown_s: f32,
    pub contact_damage_mult: f32,
    pub contact_knockback: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemiesConfig {
    pub melee_chaser: EnemyStatsConfig,
    #[serde(default = "default_lobber_stats")]
    pub lobber: EnemyStatsConfig,
    pub ranged_shooter: EnemyStatsConfig,
    pub charger: EnemyStatsConfig,
    pub charger_config: ChargerConfig,
    pub flanker: EnemyStatsConfig,
    pub sniper: EnemyStatsConfig,
    pub support_caster: EnemyStatsConfig,
    pub bomber: EnemyStatsConfig,
    pub shielder: EnemyStatsConfig,
    pub summoner: EnemyStatsConfig,
}

fn default_lobber_stats() -> EnemyStatsConfig {
    EnemyStatsConfig {
        max_hp: 36.0,
        move_speed: 112.0,
        attack_damage: 12.0,
        attack_cooldown_s: 1.25,
        aggro_range: 620.0,
        attack_range: 440.0,
        projectile_speed: 390.0,
    }
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
            attack_speed_s: FloorGains {
                floor_1: 0.04,
                floor_2: 0.06,
                floor_3: 0.07,
                floor_4: 0.08,
            },
            attack_power: FloorGains {
                floor_1: 4.0,
                floor_2: 5.0,
                floor_3: 6.0,
                floor_4: 7.0,
            },
            max_health: FloorGains {
                floor_1: 20.0,
                floor_2: 24.0,
                floor_3: 28.0,
                floor_4: 32.0,
            },
            dash_cooldown_s: FloorGains {
                floor_1: 0.08,
                floor_2: 0.10,
                floor_3: 0.12,
                floor_4: 0.14,
            },
            lifesteal: FloorGains {
                floor_1: 3.0,
                floor_2: 4.0,
                floor_3: 5.0,
                floor_4: 6.0,
            },
            crit_chance: FloorGains {
                floor_1: 0.03,
                floor_2: 0.04,
                floor_3: 0.05,
                floor_4: 0.06,
            },
            move_speed: FloorGains {
                floor_1: 18.0,
                floor_2: 24.0,
                floor_3: 30.0,
                floor_4: 36.0,
            },
            heal_base: FloorGains {
                floor_1: 24.0,
                floor_2: 30.0,
                floor_3: 36.0,
                floor_4: 42.0,
            },
            heal_hp_fraction: 0.22,
        }
    }
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
    /// Per-floor enemy unlock pools (spec §7.1). Index 0 = floor 1; each inner
    /// list = enemies NEWLY unlocked at that floor. The pool for floor N is the
    /// cumulative union of floors 1..=N. Empty -> built-in spec table is used.
    #[serde(default)]
    pub enemy_pools_by_floor: Vec<Vec<EnemyType>>,
    pub elite_chance: f32,
    pub elite_hp_mult: f32,
    pub elite_damage_mult: f32,
    pub elite_gold_bonus: u32,
    #[serde(default = "default_use_sprite_textures")]
    pub use_sprite_textures: bool,
}

fn default_use_sprite_textures() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentLevelConfig {
    pub description: String,
    #[serde(default)]
    pub params: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentConfig {
    pub id: AugmentId,
    pub category: AugmentCategory,
    pub rarity: AugmentRarity,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub upgraded_description: String,
    pub shop_cost: u32,
    #[serde(default)]
    pub levels: Vec<AugmentLevelConfig>,
}

impl AugmentConfig {
    pub fn max_stacks(&self) -> u8 {
        self.levels.len().clamp(2, 3) as u8
    }

    pub fn description_for_stacks(&self, stacks: u8) -> &str {
        let normalized = stacks.clamp(1, self.max_stacks());
        if let Some(level) = self.levels.get(normalized.saturating_sub(1) as usize) {
            return level.description.as_str();
        }
        if normalized >= 2 && !self.upgraded_description.is_empty() {
            self.upgraded_description.as_str()
        } else {
            self.description.as_str()
        }
    }

    pub fn next_description(&self, current_stacks: u8) -> &str {
        self.description_for_stacks(current_stacks.saturating_add(1))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentsConfig {
    pub augments: Vec<AugmentConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCategory {
    Melee,
    Ranged,
    Support,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillTier {
    Light,
    Medium,
    Heavy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    pub skill: SkillType,
    pub category: SkillCategory,
    pub tier: SkillTier,
    pub title: String,
    pub description: String,
    pub energy_cost: f32,
    pub cooldown_s: f32,
    #[serde(default)]
    pub consumes_all_energy: bool,
    #[serde(default)]
    pub min_energy: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub skills: Vec<SkillConfig>,
}

impl SkillsConfig {
    pub fn get(&self, skill: SkillType) -> Option<&SkillConfig> {
        self.skills.iter().find(|config| config.skill == skill)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventCategory {
    Puzzle,
    NonCombat,
    Combat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PuzzleRewardPool {
    #[default]
    None,
    Any,
    Elite,
    Legendary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PuzzleEventConfig {
    pub time_limit_s: f32,
    pub target_count: u32,
    pub lives: u32,
    pub gold_reward: u32,
    pub xp_reward: u32,
    #[serde(default)]
    pub augment_pool: PuzzleRewardPool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventChoiceConfig {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDefinitionConfig {
    pub id: String,
    pub category: EventCategory,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub puzzle: Option<PuzzleEventConfig>,
    #[serde(default)]
    pub choices: Vec<EventChoiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsConfig {
    pub events: Vec<EventDefinitionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopConfig {
    pub heal_price: u32,
    pub energy_price: u32,
    pub max_hp_price: u32,
    pub attack_power_price: u32,
    pub common_augment_price: u32,
    pub elite_augment_price: u32,
    pub legendary_augment_price: u32,
    pub augment_upgrade_price: u32,
    pub skill_price: u32,
    pub healing_potion_price: u32,
    pub energy_potion_price: u32,
    pub talisman_price: u32,
    pub refresh_first_cost: u32,
    pub refresh_base_cost: u32,
    pub refresh_increment: u32,
}

impl Default for ShopConfig {
    fn default() -> Self {
        Self {
            heal_price: 40,
            energy_price: 30,
            max_hp_price: 80,
            attack_power_price: 80,
            common_augment_price: 80,
            elite_augment_price: 150,
            legendary_augment_price: 250,
            augment_upgrade_price: 120,
            skill_price: 180,
            healing_potion_price: 60,
            energy_potion_price: 40,
            talisman_price: 120,
            refresh_first_cost: 0,
            refresh_base_cost: 30,
            refresh_increment: 15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyConfig {
    pub normal_gold: [u32; 2],
    pub elite_gold: [u32; 2],
    pub boss_gold: [u32; 2],
    pub floor_income: [u32; 2],
    pub xp_curve: Vec<u32>,
}

impl Default for EconomyConfig {
    fn default() -> Self {
        Self {
            normal_gold: [3, 6],
            elite_gold: [12, 20],
            boss_gold: [30, 50],
            floor_income: [100, 180],
            xp_curve: vec![0, 50, 120, 210, 320, 450, 600, 780, 980, 1200],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EliteAffixConfig {
    pub affix: EliteAffix,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EliteAffixesConfig {
    pub affixes: Vec<EliteAffixConfig>,
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
            death_particle_count: 8,
            hitstop_duration_s: 0.04,
            hitstop_crit_s: 0.06,
            hitstop_kill_s: 0.08,
            bar_lerp_speed: 8.0,
            screen_flash_duration_s: 0.15,
            death_scale_s: 0.25,
        }
    }
}
