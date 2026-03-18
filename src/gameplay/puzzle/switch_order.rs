use bevy::prelude::*;

use crate::constants::UI_Z;
use crate::core::assets::GameAssets;
use crate::core::input::PlayerInputState;
use crate::gameplay::map::room::{CurrentRoom, RoomId};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::player::components::Player;
use crate::gameplay::puzzle::{ActivePuzzle, PuzzleEntity, PuzzleKind};
use crate::states::RoomState;

#[derive(Component, Debug, Clone, Copy)]
pub struct Switch {
    pub index: u8,
}
