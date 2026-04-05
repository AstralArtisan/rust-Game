use bevy::prelude::*;

use crate::data::definitions::*;

#[derive(Resource, Debug, Clone)]
pub struct GameDataRegistry {
    pub player: PlayerConfig,
    pub enemies: EnemiesConfig,
    pub bosses: BossesConfig,
    pub rewards: RewardsConfig,
    pub runes: RunesConfig,
    pub curses: CursesConfig,
    pub rooms: RoomGenConfig,
    pub balance: GameBalanceConfig,
    pub augments: AugmentsConfig,
    pub audio: AudioConfig,
    pub effects: EffectsConfig,
}
