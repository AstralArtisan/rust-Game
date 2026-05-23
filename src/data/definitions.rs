use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::gameplay::augment::data::{AugmentCategory, AugmentId, AugmentRarity};
use crate::gameplay::enemy::components::{BossArchetype, EliteAffix, EnemyType};
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
    /// Per-floor scaling derived from `floor_multiplier`. Indexed by
    /// `BossArchetype`; missing archetypes fall back to `default_scaling`.
    #[serde(default)]
    pub scaling: BossScalingConfig,
    /// Floor 2 split sub-core HP formula: `base + phase * per_phase`.
    #[serde(default = "default_sub_core_base_hp")]
    pub sub_core_base_hp: f32,
    #[serde(default = "default_sub_core_hp_per_phase")]
    pub sub_core_hp_per_phase: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossScalingConfig {
    pub hp_per_floor_default: f32,
    pub hp_per_floor_cube_core: f32,
    pub move_speed_per_floor: f32,
    pub damage_per_floor: f32,
    pub cooldown_inverse_per_floor: f32,
    pub projectile_speed_per_floor: f32,
    pub base_cooldown_s: f32,
    pub min_cooldown_s: f32,
    pub aggro_range: f32,
    pub attack_range_by_archetype: Vec<(BossArchetype, f32)>,
}

impl Default for BossScalingConfig {
    fn default() -> Self {
        Self {
            hp_per_floor_default: 0.38,
            hp_per_floor_cube_core: 0.72,
            move_speed_per_floor: 0.08,
            damage_per_floor: 0.30,
            cooldown_inverse_per_floor: 0.12,
            projectile_speed_per_floor: 0.12,
            base_cooldown_s: 0.95,
            min_cooldown_s: 0.40,
            aggro_range: 900.0,
            attack_range_by_archetype: vec![
                (BossArchetype::Floor1Guardian, 42.0),
                (BossArchetype::MirrorWarden, 44.0),
                (BossArchetype::TideHunter, 52.0),
                (BossArchetype::CubeCore, 48.0),
            ],
        }
    }
}

impl BossScalingConfig {
    pub fn hp_factor_for(&self, archetype: BossArchetype) -> f32 {
        match archetype {
            BossArchetype::CubeCore => self.hp_per_floor_cube_core,
            _ => self.hp_per_floor_default,
        }
    }

    pub fn attack_range_for(&self, archetype: BossArchetype) -> f32 {
        self.attack_range_by_archetype
            .iter()
            .find(|(a, _)| *a == archetype)
            .map(|(_, range)| *range)
            .unwrap_or(42.0)
    }
}

fn default_sub_core_base_hp() -> f32 {
    40.0
}

fn default_sub_core_hp_per_phase() -> f32 {
    10.0
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
    #[serde(default = "LevelUpConfig::default_config")]
    pub levelup: LevelUpConfig,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LevelUpConfig {
    /// Flat attribute increments offered per level-up.
    pub attack_power: f32,
    pub max_health: f32,
    pub move_speed: f32,
    pub crit_chance: f32,
    /// Seconds shaved off melee attack cooldown per pick.
    pub melee_speed_s: f32,
    /// Seconds shaved off ranged attack cooldown per pick.
    pub ranged_speed_s: f32,
    /// Seconds shaved off dash cooldown per pick.
    pub dash_cooldown_s: f32,
    pub crit_cap: f32,
    pub melee_min_s: f32,
    pub ranged_min_s: f32,
    pub dash_min_s: f32,
}

impl LevelUpConfig {
    pub fn default_config() -> Self {
        Self {
            attack_power: 3.0,
            max_health: 15.0,
            move_speed: 15.0,
            crit_chance: 0.05,
            melee_speed_s: 0.06,
            ranged_speed_s: 0.04,
            dash_cooldown_s: 0.10,
            crit_cap: 0.80,
            melee_min_s: 0.15,
            ranged_min_s: 0.15,
            dash_min_s: 0.30,
        }
    }
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
    /// Normal-room enemy count by floor. Index 0 = floor 1. Falls back to
    /// `enemy_count_normal_room` if empty.
    #[serde(default)]
    pub enemy_count_by_floor: Vec<u32>,
    /// Per-floor elite spawn chance. Index 0 = floor 1. Falls back to
    /// `elite_chance` if empty.
    #[serde(default)]
    pub elite_chance_by_floor: Vec<f32>,
    /// Per-floor enemy stat growth curves (4 entries: floor 1..4). Multipliers
    /// of the per-floor `base_step = (floor_multiplier - 1) / (floor - 1)`.
    /// hp/damage/projectile add positively; cooldown subtracts (capped at 0.5).
    #[serde(default)]
    pub floor_growth_curves: Vec<FloorGrowthCurve>,
    /// Extra multipliers applied to specific enemy types when `floor >= 3`.
    #[serde(default)]
    pub enemy_type_curves: Vec<EnemyTypeCurve>,
    #[serde(default)]
    pub floor1_easing: Floor1Easing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct FloorGrowthCurve {
    pub hp: f32,
    pub damage: f32,
    pub cooldown: f32,
    pub projectile: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EnemyTypeCurve {
    pub enemy: EnemyType,
    pub hp: f32,
    pub damage: f32,
    pub cooldown: f32,
    pub projectile: f32,
    pub aggro_bonus: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Floor1Easing {
    pub room1_mult: f32,
    pub room2_mult: f32,
    pub room1_count_offset: i32,
    pub min_enemy_count: u32,
}

impl Default for Floor1Easing {
    fn default() -> Self {
        Self {
            room1_mult: 0.86,
            room2_mult: 0.93,
            room1_count_offset: -1,
            min_enemy_count: 3,
        }
    }
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
    /// ATK multiplier applied to the primary damage hit. `0.0` means the skill
    /// has no direct damage component (pure utility / buff).
    #[serde(default)]
    pub damage_mult: f32,
    /// Knockback magnitude on hit.
    #[serde(default)]
    pub knockback: f32,
    /// Radius of the round AOE hitbox. Ignored for projectile skills.
    #[serde(default)]
    pub aoe_radius: f32,
    /// Sustained duration in seconds (BladeDance / BulletBarrage / WarCry /
    /// TimeRift).
    #[serde(default)]
    pub duration_s: f32,
    /// Tick interval for repeating-damage skills (BladeDance).
    #[serde(default)]
    pub tick_interval_s: f32,
    /// Number of projectiles spawned (BulletBarrage).
    #[serde(default)]
    pub projectile_count: u32,
    /// Speed of spawned projectiles.
    #[serde(default)]
    pub projectile_speed: f32,
    /// Status modifiers keyed by name: `lifesteal_fraction`, `freeze_s`,
    /// `attack_bonus`, `move_speed_bonus`, `attack_speed_bonus`, `slow`,
    /// `invincibility_s`, etc. Reader-driven so adding a new key only requires
    /// touching RON + the consuming system.
    #[serde(default)]
    pub status: BTreeMap<String, f32>,
}

impl SkillConfig {
    pub fn status(&self, key: &str) -> f32 {
        self.status.get(key).copied().unwrap_or(0.0)
    }
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
    /// Per-purchase price increment for repeated attribute buys (design.md §6.3).
    #[serde(default)]
    pub repeat_increment: ShopRepeatIncrement,
    /// Non-floor-scaled effect magnitudes (potions, % discounts, reduction caps).
    #[serde(default)]
    pub effects: ShopEffects,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopRepeatIncrement {
    pub heal: u32,
    pub energy: u32,
    pub max_hp: u32,
    pub attack_power: u32,
}

impl Default for ShopRepeatIncrement {
    fn default() -> Self {
        Self {
            heal: 15,
            energy: 10,
            max_hp: 30,
            attack_power: 30,
        }
    }
}

/// Effect magnitudes for shop items that are NOT scaled by floor (potions,
/// flat heals, % discounts vs the reward-curve, and reduction caps).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShopEffects {
    /// Heal restored as a fraction of max HP.
    pub heal_fraction: f32,
    /// Flat energy restored by the "Restore Energy" attribute item.
    pub energy_restore: f32,
    /// Flat max-energy gain from the "Increase Energy Max" item.
    pub energy_max_gain: f32,
    /// HP fraction restored by the "Healing Potion" tool.
    pub potion_heal_fraction: f32,
    /// Flat energy restored by the "Energy Potion" tool.
    pub energy_potion_restore: f32,
    /// Discount applied to move-speed/crit reward-curve values when bought
    /// from the shop (vs from a normal levelup/reward).
    pub move_speed_factor: f32,
    pub crit_factor: f32,
    /// Hard caps on accumulated shop-only cooldown reductions.
    pub dash_cd_cap_s: f32,
    pub attack_speed_cap_s: f32,
}

impl Default for ShopEffects {
    fn default() -> Self {
        Self {
            heal_fraction: 0.30,
            energy_restore: 50.0,
            energy_max_gain: 25.0,
            potion_heal_fraction: 0.40,
            energy_potion_restore: 60.0,
            move_speed_factor: 0.75,
            crit_factor: 0.75,
            dash_cd_cap_s: 0.20,
            attack_speed_cap_s: 0.18,
        }
    }
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
            repeat_increment: ShopRepeatIncrement::default(),
            effects: ShopEffects::default(),
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
            xp_curve: vec![50, 70, 90, 110, 130, 150, 180, 200, 220],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EliteAffixConfig {
    pub affix: EliteAffix,
    pub title: String,
    pub description: String,
    /// Numerical knobs, e.g. Swift: `move_speed_mult`, `attack_cooldown_mult`,
    /// `slow_resist`; Berserk: `hp_threshold`, `damage_bonus`, ...
    #[serde(default)]
    pub params: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EliteAffixesConfig {
    pub affixes: Vec<EliteAffixConfig>,
}

impl EliteAffixesConfig {
    pub fn param(&self, affix: EliteAffix, key: &str) -> Option<f32> {
        self.affixes
            .iter()
            .find(|c| c.affix == affix)
            .and_then(|c| c.params.get(key).copied())
    }

    pub fn param_or(&self, affix: EliteAffix, key: &str, default: f32) -> f32 {
        self.param(affix, key).unwrap_or(default)
    }
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
