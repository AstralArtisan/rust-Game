use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    MainMenu,
    InGame,
    MultiplayerMenu,
    CoopMenu,
    CoopLobby,
    CoopGame,
    PvpMenu,
    PvpLobby,
    PvpGame,
    PvpResult,
    Paused,
    RewardSelect,
    AugmentSelect,
    LevelUpSelect,
    Shop,
    GameOver,
    Victory,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RoomState {
    #[default]
    Idle,
    Locked,
    Cleared,
    BossFight,
}
