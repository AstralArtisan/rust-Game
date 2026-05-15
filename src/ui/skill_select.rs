use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::gameplay::player::components::{Player, SkillSlots, SkillType};
use crate::states::GamePhase;
use crate::ui::widgets;

#[derive(Resource, Debug, Clone, Default)]
pub struct SkillChoices {
    pub options: Vec<SkillChoiceOption>,
    pub return_state: Option<GamePhase>,
}

#[derive(Debug, Clone)]
pub struct SkillChoiceOption {
    pub skill: SkillType,
    pub title: String,
    pub description: String,
    pub energy_cost: f32,
    pub cooldown_s: f32,
}

#[derive(Component)]
pub struct SkillSelectUi;

#[derive(Component)]
pub struct SkillButton {
    pub index: usize,
}

pub fn setup_skill_select_ui(
    mut commands: Commands,
    assets: Res<GameAssets>,
    choices: Res<SkillChoices>,
) {
    commands
        .spawn((
            widgets::root_node(),
            SkillSelectUi,
            Name::new("SkillSelectRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::panel_node(Color::srgba(0.02, 0.03, 0.06, 0.94)))
                .with_children(|panel| {
                    panel.spawn(widgets::title_text(&assets, "获得终结技", 30.0));
                    panel.spawn(widgets::title_text(
                        &assets,
                        "按 1 / 2 / 3 选择一项终结技",
                        16.0,
                    ));
                    panel
                        .spawn(NodeBundle {
                            style: Style {
                                column_gap: Val::Px(16.0),
                                align_items: AlignItems::FlexStart,
                                margin: UiRect::top(Val::Px(12.0)),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|row| {
                            for (index, option) in choices.options.iter().enumerate() {
                                spawn_skill_card(row, &assets, index, option);
                            }
                        });
                });
        });
}

fn spawn_skill_card(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    index: usize,
    option: &SkillChoiceOption,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(230.0),
                    min_height: Val::Px(170.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.95)),
                border_color: BorderColor(Color::srgb(0.30, 0.72, 0.95)),
                ..default()
            },
            SkillButton { index },
        ))
        .with_children(|card| {
            card.spawn(TextBundle::from_section(
                format!("[{}]", index + 1),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 14.0,
                    color: Color::srgb(0.62, 0.68, 0.78),
                },
            ));
            card.spawn(TextBundle::from_section(
                format!("{} 能量 / {:.0}s CD", option.energy_cost, option.cooldown_s),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 13.0,
                    color: Color::srgb(0.30, 0.72, 0.95),
                },
            ));
            card.spawn(TextBundle::from_section(
                &option.title,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ));
            card.spawn(TextBundle::from_section(
                &option.description,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 14.0,
                    color: Color::srgb(0.78, 0.82, 0.88),
                },
            ));
        });
}

pub fn skill_select_input(
    keys: Res<ButtonInput<KeyCode>>,
    choices: Res<SkillChoices>,
    mut player_q: Query<&mut SkillSlots, With<Player>>,
    mut next_state: ResMut<NextState<GamePhase>>,
    button_q: Query<(&Interaction, &SkillButton), Changed<Interaction>>,
) {
    let mut picked: Option<usize> = None;
    if keys.just_pressed(KeyCode::Digit1) || keys.just_pressed(KeyCode::Numpad1) {
        picked = Some(0);
    } else if keys.just_pressed(KeyCode::Digit2) || keys.just_pressed(KeyCode::Numpad2) {
        picked = Some(1);
    } else if keys.just_pressed(KeyCode::Digit3) || keys.just_pressed(KeyCode::Numpad3) {
        picked = Some(2);
    }
    for (interaction, button) in &button_q {
        if *interaction == Interaction::Pressed {
            picked = Some(button.index);
        }
    }

    let Some(index) = picked else { return };
    let Some(option) = choices.options.get(index) else {
        return;
    };
    if let Ok(mut slots) = player_q.get_single_mut() {
        slots.equip_first_available(option.skill);
    }
    next_state.set(choices.return_state.unwrap_or(GamePhase::Playing));
}

pub fn cleanup_skill_select_ui(mut commands: Commands, q: Query<Entity, With<SkillSelectUi>>) {
    for entity in &q {
        commands.entity(entity).despawn_recursive();
    }
}
