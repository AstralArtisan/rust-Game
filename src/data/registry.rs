use bevy::prelude::*;

use crate::data::definitions::*;
use crate::gameplay::augment::data::AugmentId;

#[derive(Resource, Debug, Clone)]
pub struct GameDataRegistry {
    pub player: PlayerConfig,
    pub enemies: EnemiesConfig,
    pub bosses: BossesConfig,
    #[allow(dead_code)]
    pub rewards: RewardsConfig,
    pub rooms: RoomGenConfig,
    pub balance: GameBalanceConfig,
    pub augments: AugmentsConfig,
    #[allow(dead_code)]
    pub skills: SkillsConfig,
    #[allow(dead_code)]
    pub events: EventsConfig,
    #[allow(dead_code)]
    pub shop: ShopConfig,
    #[allow(dead_code)]
    pub economy: EconomyConfig,
    #[allow(dead_code)]
    pub elite_affixes: EliteAffixesConfig,
    #[allow(dead_code)]
    pub audio: AudioConfig,
    pub effects: EffectsConfig,
}

impl GameDataRegistry {
    /// Look up a `params` value from `augments.ron` for the given augment id
    /// at the given stack level (1-indexed: Lv1 / Lv2 / Lv3).
    ///
    /// Returns `None` if the augment, level, or key is missing — callers can
    /// then fall back to the compile-time defaults in `gameplay::augment::tuning`.
    pub fn augment_param(&self, id: AugmentId, stacks: u8, key: &str) -> Option<f32> {
        if stacks == 0 {
            return None;
        }
        let cfg = self
            .augments
            .augments
            .iter()
            .find(|augment| augment.id == id)?;
        let level_idx = (stacks.saturating_sub(1) as usize).min(cfg.levels.len().saturating_sub(1));
        cfg.levels.get(level_idx)?.params.get(key).copied()
    }

    /// Variant of `augment_param` that returns `default` when the key isn't
    /// configured, so call sites stay terse.
    pub fn augment_param_or(&self, id: AugmentId, stacks: u8, key: &str, default: f32) -> f32 {
        self.augment_param(id, stacks, key).unwrap_or(default)
    }
}
