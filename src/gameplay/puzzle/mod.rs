pub mod pressure_plate;
pub mod switch_order;
pub mod trap;

use bevy::prelude::*;

use crate::gameplay::map::room::RoomId;
use crate::core::assets::GameAssets;
use crate::states::AppState;
use crate::utils::rng::GameRng;

pub struct PuzzlePlugin;

impl Plugin for PuzzlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActivePuzzle>().add_systems(
            Update,
            (
                pressure_plate::pressure_plate_system,
                switch_order::switch_order_system,
                trap::trap_system,
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}
