pub mod animation;
pub mod combat;
pub mod combo;
pub mod components;
pub mod dash;
pub mod systems;

use bevy::prelude::*;

use crate::coop::net::is_coop_authority;
use crate::coop::runtime::is_coop_simulation_active;
use crate::states::{AppState, GamePhase};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), systems::spawn_player)
            .add_systems(
                Update,
                systems::push_local_input_to_players
                    .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
            )
            .add_systems(
                Update,
                (
                    systems::player_invincibility_system,
                    systems::player_buff_tick_system,
                    systems::player_move_system,
                    systems::player_facing_system,
                    combat::player_attack_input_system,
                    combat::player_ranged_input_system,
                    combat::update_attack_cooldowns,
                    combat::update_delayed_ranged_shots,
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
                            )
                            .and_then(in_state(GamePhase::Playing)),
                    ),
            );
        app.add_systems(
            Update,
            systems::player_death_system
                .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
        );
        // 纯视觉系统：不需要 authority，client 端也需要驱动 slash 动画和自动消亡
        app.add_systems(
            Update,
            combat::update_melee_slash_effects.run_if(
                in_state(AppState::InGame)
                    .or_else(in_state(AppState::CoopGame))
                    .and_then(in_state(GamePhase::Playing)),
            ),
        );
    }
}
