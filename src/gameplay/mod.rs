pub mod augment;
pub mod combat;
pub mod drops;
pub mod effects;
pub mod enemy;
pub mod event_room;
pub mod map;
pub mod player;
pub mod progression;
pub mod puzzle;
pub mod rewards;
pub mod session_core;
pub mod shop;
pub mod skills;

use bevy::prelude::*;

use crate::states::{AppState, GamePhase};

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            map::MapPlugin,
            progression::ProgressionPlugin,
            combat::CombatPlugin,
            player::PlayerPlugin,
            skills::SkillsPlugin,
            enemy::EnemyPlugin,
            rewards::RewardsPlugin,
            effects::EffectsPlugin,
            puzzle::PuzzlePlugin,
            event_room::EventRoomPlugin,
            shop::ShopPlugin,
            drops::DropPlugin,
            augment::AugmentPlugin,
        ))
        .add_systems(OnEnter(AppState::MainMenu), map::cleanup_ingame_world)
        .add_systems(OnEnter(GamePhase::GameOver), map::cleanup_ingame_world)
        .add_systems(OnEnter(GamePhase::Victory), map::cleanup_ingame_world);
    }
}
