use bevy::app::AppExit;
use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::core::save::{LoadRequestEvent, SaveRequestEvent};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::player::components::Player;
use crate::states::{AppState, GamePhase};
use crate::ui::character_panel::{self, CharacterSummaryItem};
use crate::ui::widgets;

#[derive(Component)]
pub struct PauseUi;

#[derive(Component)]
pub struct PauseActionButton {
    action: PauseAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PauseAction {
    Resume,
    Save,
    Load,
    MainMenu,
    Quit,
}

pub fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    phase: Option<Res<State<GamePhase>>>,
    mut next: ResMut<NextState<GamePhase>>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }
    let Some(phase) = phase else {
        return;
    };
    match phase.get() {
        GamePhase::Playing => next.set(GamePhase::Paused),
        GamePhase::Paused => next.set(GamePhase::Playing),
        _ => {}
    }
}

pub fn setup_pause_menu(
    mut commands: Commands,
    assets: Res<GameAssets>,
    data: Option<Res<GameDataRegistry>>,
    summary_q: Query<CharacterSummaryItem<'_>, With<Player>>,
) {
    let summary = character_panel::character_summary_from_query(&summary_q, data.as_deref());
    commands
        .spawn((
            widgets::overlay_root_node(),
            PauseUi,
            Name::new("PauseRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::responsive_panel_node(78.0, 85.0))
                .with_children(|panel| {
                    panel.spawn(widgets::title_text(&assets, "游戏暂停", 32.0));
                    panel
                        .spawn(widgets::content_row_node())
                        .with_children(|row| {
                            row.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Px(240.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    row_gap: Val::Px(8.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Stretch,
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(widgets::section_color()),
                                border_color: BorderColor(widgets::gold_color()),
                                ..default()
                            })
                                .with_children(|menu| {
                                    menu.spawn(widgets::section_heading(&assets, "操作"));
                                    spawn_pause_button(
                                        menu,
                                        &assets,
                                        "继续游戏",
                                        PauseAction::Resume,
                                    );
                                    spawn_pause_button(
                                        menu,
                                        &assets,
                                        "保存进度 (F5)",
                                        PauseAction::Save,
                                    );
                                    spawn_pause_button(
                                        menu,
                                        &assets,
                                        "读取进度 (F9)",
                                        PauseAction::Load,
                                    );
                                    spawn_pause_button(
                                        menu,
                                        &assets,
                                        "回到主菜单",
                                        PauseAction::MainMenu,
                                    );
                                    spawn_pause_button(
                                        menu,
                                        &assets,
                                        "退出游戏",
                                        PauseAction::Quit,
                                    );
                                    menu.spawn(widgets::muted_text(
                                        &assets,
                                        "ESC 继续 · M 主菜单 · Q 退出",
                                        11.0,
                                    ));
                                });
                            character_panel::spawn_character_summary(row, &assets, &summary);
                        });
                });
        });
}

pub fn pause_menu_keyboard_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_phase: ResMut<NextState<GamePhase>>,
    mut next_app: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
    mut save_events: EventWriter<SaveRequestEvent>,
    mut load_events: EventWriter<LoadRequestEvent>,
    mut button_q: Query<
        (&Interaction, &PauseActionButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
) {
    let mut action = None;
    if keyboard.just_pressed(KeyCode::Escape) {
        action = Some(PauseAction::Resume);
    } else if keyboard.just_pressed(KeyCode::KeyM) {
        action = Some(PauseAction::MainMenu);
    } else if keyboard.just_pressed(KeyCode::KeyQ) {
        action = Some(PauseAction::Quit);
    }

    for (interaction, button, mut color) in &mut button_q {
        match *interaction {
            Interaction::Hovered => color.0 = widgets::button_hover_color(),
            Interaction::None => color.0 = widgets::button_base_color(),
            Interaction::Pressed => action = Some(button.action),
        }
    }

    match action {
        Some(PauseAction::Resume) => next_phase.set(GamePhase::Playing),
        Some(PauseAction::Save) => {
            save_events.send(SaveRequestEvent);
        }
        Some(PauseAction::Load) => {
            load_events.send(LoadRequestEvent);
        }
        Some(PauseAction::MainMenu) => next_app.set(AppState::MainMenu),
        Some(PauseAction::Quit) => {
            let _ = exit.send(AppExit::Success);
        }
        None => {}
    }
}

fn spawn_pause_button(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    label: &str,
    action: PauseAction,
) {
    parent
        .spawn((widgets::button_bundle(), PauseActionButton { action }))
        .with_children(|button| {
            button.spawn(widgets::title_text(assets, label, 16.0));
        });
}

pub fn cleanup_pause_menu(mut commands: Commands, q: Query<Entity, With<PauseUi>>) {
    for entity in &q {
        commands.entity(entity).despawn_recursive();
    }
}
