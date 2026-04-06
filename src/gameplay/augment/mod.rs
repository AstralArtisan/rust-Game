pub mod data;
pub mod effects;

use bevy::prelude::*;

use crate::coop::net::is_coop_authority;
use crate::coop::runtime::is_coop_simulation_active;
use crate::states::AppState;

pub struct AugmentPlugin;

impl Plugin for AugmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            effects::dash_energy_system
                .after(crate::gameplay::combat::damage::apply_damage_events)
                .run_if(
                    in_state(AppState::InGame).or_else(
                        in_state(AppState::CoopGame)
                            .and_then(is_coop_authority)
                            .and_then(is_coop_simulation_active),
                    ),
                ),
        );
    }
}
