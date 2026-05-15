use bevy::prelude::*;
use bevy::state::state::FreelyMutableState;
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
}

/// In-session phase, layered on top of `AppState::InGame` / `AppState::CoopGame`.
///
/// Manual `SubStates` impl (rather than the derive macro) because the derive
/// only supports a single source variant, but this phase must exist for BOTH
/// `InGame` and `CoopGame`. Variant names intentionally mirror the former
/// `AppState` overlay variants to keep the migration mechanical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GamePhase {
    #[default]
    Playing,
    Paused,
    RewardSelect,
    AugmentSelect,
    SkillSelect,
    LevelUpSelect,
    Shop,
    EventRoom,
    GameOver,
    Victory,
}

impl SubStates for GamePhase {
    type SourceStates = Option<AppState>;

    fn should_exist(sources: Option<AppState>) -> Option<Self> {
        match sources {
            Some(AppState::InGame) | Some(AppState::CoopGame) => Some(Self::Playing),
            _ => None,
        }
    }
}

impl States for GamePhase {
    const DEPENDENCY_DEPTH: usize =
        <GamePhase as SubStates>::SourceStates::SET_DEPENDENCY_DEPTH + 1;
}

impl FreelyMutableState for GamePhase {}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RoomState {
    #[default]
    Idle,
    Locked,
    Cleared,
    BossFight,
}
