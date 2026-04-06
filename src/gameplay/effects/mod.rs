pub mod afterimage;
pub mod damage_numbers;
pub mod death_effect;
pub mod flash;
pub mod hitstop;
pub mod particles;
pub mod screen_flash;
pub mod screen_shake;

use bevy::prelude::*;

use crate::states::AppState;

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<screen_shake::ScreenShakeRequest>()
            .init_resource::<hitstop::HitStopState>()
            .add_systems(
                OnEnter(AppState::InGame),
                screen_flash::spawn_screen_flash_overlay,
            )
            .add_systems(
                OnEnter(AppState::CoopGame),
                screen_flash::spawn_screen_flash_overlay,
            )
            .add_systems(
                Update,
                (
                    flash::update_flash_effect,
                    particles::update_particles,
                    particles::update_expanding_rings,
                    afterimage::update_afterimages,
                    damage_numbers::update_damage_numbers,
                    hitstop::hitstop_receive_system,
                    hitstop::hitstop_update_system,
                    screen_flash::screen_flash_receive_system,
                    screen_flash::screen_flash_update_system,
                    death_effect::death_effect_system
                        .after(crate::gameplay::combat::damage::apply_damage_events)
                        .before(crate::gameplay::enemy::systems::enemy_death_system),
                )
                    .run_if(
                        in_state(AppState::InGame)
                            .or_else(in_state(AppState::CoopGame))
                            .or_else(in_state(AppState::PvpGame)),
                    ),
            );
    }
}
