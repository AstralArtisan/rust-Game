pub mod augment;
pub mod combat;
pub mod curse;
pub mod drops;
pub mod effects;
pub mod enemy;
pub mod event_room;
pub mod map;
pub mod player;
pub mod progression;
pub mod puzzle;
pub mod rewards;
pub mod rune;
pub mod session_core;
pub mod shop;
pub mod skills;

use bevy::prelude::*;

use crate::states::AppState;

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
        ))
        .add_systems(OnEnter(AppState::MainMenu), map::cleanup_ingame_world)
        .add_systems(OnEnter(AppState::GameOver), map::cleanup_ingame_world)
        .add_systems(OnEnter(AppState::Victory), map::cleanup_ingame_world);
    }
}
