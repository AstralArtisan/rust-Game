pub mod doors;
pub mod generator;
pub mod room;
pub mod tiles;
pub mod transitions;

use bevy::prelude::*;
use bevy::utils::HashSet;
use lightyear::prelude::Replicated;

use crate::coop::net::is_coop_authority;
use crate::coop::runtime::is_coop_simulation_active;
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId, RoomType};
use crate::gameplay::map::transitions::RoomTransition;
use crate::gameplay::player::components::{Gold, Player};
use crate::states::{AppState, RoomState};
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
                (track_visited_rooms, reward_room_gold_bonus_on_enter).run_if(
                    in_state(AppState::InGame).or_else(
                        in_state(AppState::CoopGame)
                            .and_then(is_coop_authority)
                            .and_then(is_coop_simulation_active),
                    ),
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

pub fn reward_room_gold_bonus_on_enter(
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    mut seen: ResMut<RewardRoomGoldBonusSeen>,
    mut gold_q: Query<&mut Gold, (With<Player>, Without<Replicated>)>,
) {
    let (Some(layout), Some(current)) = (layout, current) else {
        return;
    };
    if layout.is_changed() {
        seen.0.clear();
    }
    if !current.is_changed() && !layout.is_changed() {
        return;
    }
    let Some(room) = layout.room(current.0) else {
        return;
    };
    if room.room_type != RoomType::Reward {
        return;
    }
    if !seen.0.insert(current.0) {
        return;
    }
    for mut gold in &mut gold_q {
        gold.0 = gold.0.saturating_add(15);
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
