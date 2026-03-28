pub mod animation;
pub mod combat;
pub mod combo;
pub mod components;
pub mod dash;
pub mod skills;
pub mod systems;

use bevy::prelude::*;

use crate::coop::net::is_coop_authority;
use crate::coop::runtime::is_coop_simulation_active;
use crate::states::AppState;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), systems::spawn_player)
            .add_systems(
                Update,
                systems::push_local_input_to_players.run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    systems::player_invincibility_system,
                    systems::player_move_system,
                    systems::player_facing_system,
                    combat::player_attack_input_system,
                    combat::player_ranged_input_system,
                    combat::update_attack_cooldowns,
                    combat::update_delayed_ranged_shots,
                    combat::update_melee_slash_effects,
                    dash::player_dash_input_system,
                    dash::update_dash_state,
                    combo::update_combo_state,
                    animation::update_player_animation_state,
                    animation::animate_player_sprite,
                )
                    .run_if(
                        in_state(AppState::InGame)
                            .or_else(
                                in_state(AppState::CoopGame)
                                    .and_then(is_coop_authority)
                                    .and_then(is_coop_simulation_active),
                            ),
                    ),
            );
        app.add_systems(
            Update,
            systems::player_death_system.run_if(in_state(AppState::InGame)),
        );
    }
}
