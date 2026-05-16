use bevy::prelude::*;

use crate::data::definitions::*;

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
