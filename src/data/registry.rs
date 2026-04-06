#![allow(dead_code)]

use bevy::prelude::*;

use crate::data::definitions::*;

#[derive(Resource, Debug, Clone)]
pub struct GameDataRegistry {
    pub player: PlayerConfig,
    pub enemies: EnemiesConfig,
    pub bosses: BossesConfig,
    #[allow(dead_code)]
    pub rewards: RewardsConfig,
    pub runes: RunesConfig,
    pub curses: CursesConfig,
    pub rooms: RoomGenConfig,
    pub balance: GameBalanceConfig,
    pub augments: AugmentsConfig,
    pub audio: AudioConfig,
    pub effects: EffectsConfig,
}
