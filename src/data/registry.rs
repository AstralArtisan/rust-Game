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
    pub audio: AudioConfig,
    pub effects: EffectsConfig,
}
