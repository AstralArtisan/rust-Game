pub mod difficulty;
pub mod experience;
pub mod floor;
pub mod stats;

use bevy::prelude::*;

use crate::states::AppState;

pub struct ProgressionPlugin;

impl Plugin for ProgressionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<experience::XpGainEvent>()
            .add_event::<experience::LevelUpEvent>()
            .init_resource::<experience::PendingLevelUps>()
            .add_systems(OnEnter(AppState::InGame), floor::setup_floor)
            .add_systems(
                Update,
                (
                    floor::complete_floor,
                    stats::update_run_stats,
                    experience::process_xp_gains,
                    experience::handle_levelup_event.after(experience::process_xp_gains),
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}
