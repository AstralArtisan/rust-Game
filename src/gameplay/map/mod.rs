pub mod doors;
pub mod generator;
pub mod room;
pub mod tiles;
pub mod transitions;

use bevy::prelude::*;
use bevy::utils::HashSet;

use crate::coop::net::is_coop_authority;
use crate::coop::runtime::is_coop_simulation_active;
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId};
use crate::gameplay::map::transitions::RoomTransition;
use crate::states::{AppState, GamePhase, RoomState};
use crate::utils::entity::safe_despawn_recursive;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VisitedRooms>()
            .init_resource::<RewardRoomGoldBonusSeen>()
            .add_plugins((
                transitions::TransitionsPlugin,
                doors::DoorsPlugin,
                tiles::TilesPlugin,
            ))
            .add_systems(
                OnEnter(AppState::InGame),
                generator::generate_and_spawn_floor,
            )
            .add_systems(
                Update,
                track_visited_rooms.run_if(
                    in_state(AppState::InGame)
                        .or_else(
                            in_state(AppState::CoopGame)
                                .and_then(is_coop_authority)
                                .and_then(is_coop_simulation_active),
                        )
                        .and_then(in_state(GamePhase::Playing)),
                ),
            );
    }
}

#[derive(Component)]
pub struct InGameEntity;

#[derive(Resource, Debug, Default, Clone)]
pub struct VisitedRooms(pub HashSet<RoomId>);

#[derive(Resource, Debug, Default, Clone)]
pub struct RewardRoomGoldBonusSeen(pub HashSet<RoomId>);

pub fn track_visited_rooms(mut visited: ResMut<VisitedRooms>, current: Option<Res<CurrentRoom>>) {
    let Some(current) = current else {
        return;
    };
    if current.is_changed() {
        visited.0.insert(current.0);
    }
}

pub fn cleanup_ingame_world(mut commands: Commands, q: Query<Entity, With<InGameEntity>>) {
    for e in &q {
        safe_despawn_recursive(&mut commands, e);
    }
    commands.remove_resource::<FloorLayout>();
    commands.remove_resource::<CurrentRoom>();
    commands.remove_resource::<RoomTransition>();
    commands.remove_resource::<RoomState>();
}
