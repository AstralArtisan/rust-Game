pub mod augment_select;
pub mod cursor;
pub mod game_over;
pub mod hud;
pub mod levelup_select;
pub mod menu;
pub mod notifications;
pub mod pause;
pub mod reward_select;
pub mod shop;
pub mod tutorial;
pub mod widgets;

use bevy::prelude::*;

use crate::gameplay::effects::screen_flash::clear_screen_flash;
use crate::states::AppState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(tutorial::TutorialPlugin)
            .add_systems(
                Update,
                (
                    cursor::ensure_cursor_visuals,
                    cursor::sync_window_cursor_visibility,
                    cursor::update_custom_cursor,
                    cursor::update_crosshair,
                ),
            )
            .add_systems(OnEnter(AppState::MainMenu), menu::setup_main_menu)
            .add_systems(
                Update,
                menu::menu_button_system.run_if(in_state(AppState::MainMenu)),
            )
            .add_systems(OnExit(AppState::MainMenu), menu::cleanup_main_menu)
            .add_systems(Update, notifications::ensure_notification_root)
            .add_systems(Update, notifications::handle_achievement_notifications)
            .add_systems(Update, notifications::update_notifications)
            .add_systems(OnEnter(AppState::InGame), hud::setup_hud)
            .add_systems(OnEnter(AppState::CoopGame), hud::setup_hud)
            .add_systems(OnEnter(AppState::RewardSelect), hud::setup_hud)
            .add_systems(
                Update,
                (
                    hud::update_health_bar,
                    hud::update_health_text,
                    hud::update_rune_and_curse_ui,
                    hud::update_gold_text,
                    hud::update_energy_text,
                    hud::update_dash_cooldown_ui,
                    hud::update_skill_bar_ui,
                    hud::update_floor_text,
                    hud::update_room_text,
                    hud::update_enemy_count_text,
                    hud::update_hint_text,
                    hud::update_boss_health_bar,
                    hud::update_stage_progress,
                )
                    .run_if(
                        in_state(AppState::InGame)
                            .or_else(in_state(AppState::RewardSelect))
                            .or_else(in_state(AppState::CoopGame)),
                    ),
            )
            .add_systems(OnExit(AppState::InGame), hud::cleanup_hud)
            .add_systems(OnExit(AppState::CoopGame), hud::cleanup_hud)
            .add_systems(OnExit(AppState::RewardSelect), hud::cleanup_hud)
            .add_systems(Update, pause::toggle_pause)
            .add_systems(OnEnter(AppState::Paused), pause::setup_pause_menu)
            .add_systems(
                Update,
                (
                    pause::pause_menu_keyboard_system,
                    pause::update_pause_character_panel,
                )
                    .run_if(in_state(AppState::Paused)),
            )
            .add_systems(OnExit(AppState::Paused), pause::cleanup_pause_menu)
            .add_systems(OnEnter(AppState::Shop), shop::setup_shop_ui)
            .add_systems(
                Update,
                shop::shop_ui_input_system.run_if(in_state(AppState::Shop)),
            )
            .add_systems(
                Update,
                shop::update_shop_ui.run_if(in_state(AppState::Shop)),
            )
            .add_systems(OnExit(AppState::Shop), shop::cleanup_shop_ui)
            // Augment select
            .init_resource::<augment_select::AugmentChoices>()
            .add_systems(OnEnter(AppState::AugmentSelect), (augment_select::setup_augment_select_ui, clear_screen_flash))
            .add_systems(Update, augment_select::augment_select_input.run_if(in_state(AppState::AugmentSelect)))
            .add_systems(OnExit(AppState::AugmentSelect), augment_select::cleanup_augment_select_ui)
            // Level-up select
            .init_resource::<levelup_select::LevelUpChoices>()
            .add_systems(OnEnter(AppState::LevelUpSelect), (levelup_select::setup_levelup_ui, clear_screen_flash))
            .add_systems(Update, levelup_select::levelup_input.run_if(in_state(AppState::LevelUpSelect)))
            .add_systems(OnExit(AppState::LevelUpSelect), levelup_select::cleanup_levelup_ui)
            //
            .add_systems(
                OnEnter(AppState::GameOver),
                game_over::setup_game_over_screen,
            )
            .add_systems(OnEnter(AppState::Victory), game_over::setup_victory_screen)
            .add_systems(
                Update,
                game_over::end_screen_input_system
                    .run_if(in_state(AppState::GameOver).or_else(in_state(AppState::Victory))),
            );
    }
}
