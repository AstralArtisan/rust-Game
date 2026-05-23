#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enemy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnemyType {
    MeleeChaser,
    Lobber,
    RangedShooter,
    Charger,
    Flanker,
    Sniper,
    SupportCaster,
    Bomber,
    Shielder,
    Summoner,
    Boss,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnemyKind(pub EnemyType);

#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyStats {
    pub max_hp: f32,
    pub move_speed: f32,
    pub attack_damage: f32,
    pub attack_cooldown_s: f32,
    pub aggro_range: f32,
    pub attack_range: f32,
    pub projectile_speed: f32,
}

#[derive(Component, Debug, Clone)]
pub struct EnemyAttackCooldown {
    pub timer: Timer,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Elite;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EliteAffix {
    Swift,
    Splitting,
    Shielded,
    Vampiric,
    Berserk,
    Teleporting,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct EliteAffixMarker(pub EliteAffix);

#[derive(Component, Debug, Clone)]
pub struct EliteAffixes(pub Vec<EliteAffix>);

impl EliteAffixes {
    pub fn contains(&self, affix: EliteAffix) -> bool {
        self.0.contains(&affix)
    }
}

#[derive(Component)]
pub struct EliteAffixLabel;

#[derive(Component)]
pub struct EliteGlow;

impl EliteAffix {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Swift => "迅捷",
            Self::Splitting => "分裂",
            Self::Shielded => "护盾",
            Self::Vampiric => "吸血",
            Self::Berserk => "狂暴",
            Self::Teleporting => "闪现",
        }
    }
    pub fn color(&self) -> Color {
        match self {
            Self::Swift => Color::srgb(0.3, 0.9, 1.0),
            Self::Splitting => Color::srgb(0.5, 1.0, 0.5),
            Self::Shielded => Color::srgb(0.7, 0.7, 1.0),
            Self::Vampiric => Color::srgb(1.0, 0.3, 0.3),
            Self::Berserk => Color::srgb(1.0, 0.5, 0.0),
            Self::Teleporting => Color::srgb(0.8, 0.4, 1.0),
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ShieldedAffixState {
    /// Damage taken is multiplied by `1.0 - damage_reduction` (0..1).
    /// design.md §7.2: Shielded affix reduces incoming damage by 25%.
    pub damage_reduction: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct BerserkAffixState {
    pub active: bool,
}

#[derive(Component, Debug, Clone)]
pub struct TeleportAffixTimer {
    pub timer: Timer,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct BossPhase(pub u8);

#[derive(Component, Debug, Clone)]
pub struct BossPatternTimer(pub Timer);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BossArchetype {
    Floor1Guardian,
    MirrorWarden,
    TideHunter,
    CubeCore,
}

impl BossArchetype {
    pub fn from_floor(floor: u32) -> Self {
        match floor {
            0 | 1 => Self::Floor1Guardian,
            2 => Self::MirrorWarden,
            3 => Self::TideHunter,
            _ => Self::CubeCore,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct BossCycleState {
    pub step: u8,
    pub anchor_index: usize,
    pub rotation: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct BossSummoned;

/// Marker for split spawns from elite Splitting affix — these should not drop loot.
#[derive(Component, Debug, Clone, Copy)]
pub struct SplitSpawn;

/// Boss face direction. Hits from the guarded front arc deal reduced damage.
#[derive(Component, Debug, Clone, Copy)]
pub struct BossDirectionalDefense {
    pub facing: Vec2,
}

/// Marker for the visual shield indicator child entity on Floor1Guardian.
#[derive(Component, Debug, Clone, Copy)]
pub struct GuardianShieldIndicator;

/// MirrorWarden decoy. Phase 3 decoys have 20% of the boss HP.
#[derive(Component, Debug, Clone)]
pub struct BossDecoy {
    pub lifetime: Timer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TideHunterPhase {
    Stalk,
    Telegraph,
    ShadowDash,
    Reposition,
    Stunned,
}

#[derive(Component, Debug, Clone)]
pub struct TideHunterState {
    pub phase: TideHunterPhase,
    pub timer: Timer,
    pub dash_target: Vec2,
    pub dash_start: Vec2,
    pub dashes_remaining: u8,
    pub dashes_per_cycle: u8,
    pub shadow_duration_s: f32,
    pub stalk_duration_s: f32,
    pub reposition_duration_s: f32,
    pub contact_hit_cooldown: Timer,
    pub parry_window_active: bool,
}

#[derive(Component, Debug, Clone)]
pub struct ShadowTrail {
    pub lifetime: Timer,
    pub damage: f32,
    pub radius: f32,
}

/// CubeCore satellite core that orbits the main boss body.
#[derive(Component, Debug, Clone)]
pub struct BossSubCore {
    pub boss_entity: Entity,
    pub orbit_angle: f32,
    pub orbit_speed: f32,
}

/// CubeCore shield state. The main body is immune while any cores remain.
#[derive(Component, Debug, Clone, Copy)]
pub struct BossCoreShield {
    pub cores_alive: u8,
}

#[derive(Component, Debug, Clone)]
pub struct EnemyBuffState {
    pub speed_mult: f32,
    pub cooldown_mult: f32,
    pub timer: Timer,
}

#[derive(Component, Debug, Clone)]
pub struct ChargerState {
    pub phase: ChargerPhase,
    pub timer: Timer,
    pub dir: Vec2,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ChargerStunVisual {
    pub spin: f32,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ChargerWindupVisual {
    pub pulse: f32,
}

#[derive(Component, Debug, Clone)]
pub struct FlankerState {
    pub phase: FlankerPhase,
    pub timer: Timer,
    pub dir: Vec2,
    pub strafe_sign: f32,
    pub repath_timer: Timer,
}

#[derive(Component, Debug, Clone)]
pub struct SniperState {
    pub phase: SniperPhase,
    pub timer: Timer,
    pub aim_dir: Vec2,
}

#[derive(Component, Debug, Clone)]
pub struct BomberState {
    pub phase: BomberPhase,
    pub timer: Timer,
    pub explosion_radius: f32,
    pub explosion_damage: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ShielderState {
    pub facing: Vec2,
    pub shield_half_angle: f32,
}

#[derive(Component, Debug, Clone)]
pub struct SummonerState {
    pub summon_timer: Timer,
    pub max_active_summons: u8,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct SummonedBy(pub Entity);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargerPhase {
    Idle,
    Windup,
    Charging,
    Stunned,
    Cooldown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlankerPhase {
    Stalk,
    Windup,
    Lunging,
    Recover,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SniperPhase {
    Idle,
    Aiming,
    Recover,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BomberPhase {
    Approach,
    Fuse,
    Exploded,
}
