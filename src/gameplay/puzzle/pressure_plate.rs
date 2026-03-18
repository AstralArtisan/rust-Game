use bevy::prelude::*;

use crate::constants::UI_Z;
use crate::core::assets::GameAssets;
use crate::gameplay::map::room::{CurrentRoom, RoomId};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::player::components::Player;
use crate::gameplay::puzzle::{ActivePuzzle, PuzzleEntity, PuzzleKind};
use crate::states::RoomState;

#[derive(Component, Debug, Clone)]
pub struct PressurePlate {
    pub required_s: f32,
    pub progress_s: f32,
    pub radius: f32,
}
