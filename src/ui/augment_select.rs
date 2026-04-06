use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory, AugmentRarity};
use crate::gameplay::player::components::Player;
use crate::states::AppState;
use crate::ui::widgets;

/// Resource: holds the augment choices offered to the player.
#[derive(Resource, Debug, Clone, Default)]
pub struct AugmentChoices {
    pub options: Vec<AugmentChoiceOption>,
    /// State to return to after selection (InGame or CoopGame).
    pub return_state: Option<AppState>,
}

#[derive(Debug, Clone)]
pub struct AugmentChoiceOption {
    pub id: AugmentId,
    pub title: String,
    pub description: String,
    pub rarity: AugmentRarity,
    pub is_upgrade: bool,
}

#[derive(Component)]
pub struct AugmentSelectUi;

#[derive(Component)]
pub struct AugmentButton {
    pub index: usize,
}

fn rarity_color(rarity: AugmentRarity) -> Color {
    match rarity {
        AugmentRarity::Common => Color::srgb(0.75, 0.78, 0.82),
        AugmentRarity::Elite => Color::srgb(0.35, 0.55, 0.95),
        AugmentRarity::Legendary => Color::srgb(0.95, 0.75, 0.20),
    }
}

fn rarity_label(rarity: AugmentRarity) -> &'static str {
    match rarity {
        AugmentRarity::Common => "普通",
        AugmentRarity::Elite => "精英",
        AugmentRarity::Legendary => "传说",
    }
}

pub fn setup_augment_select_ui(
    mut commands: Commands,
    assets: Res<GameAssets>,
    choices: Res<AugmentChoices>,
) {
    commands
        .spawn((
            widgets::root_node(),
            AugmentSelectUi,
            Name::new("AugmentSelectRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::panel_node(Color::srgba(0.02, 0.02, 0.06, 0.94)))
                .with_children(|panel| {
                    panel.spawn(widgets::title_text(&assets, "获得强化", 30.0));
                    panel.spawn(widgets::title_text(
                        &assets,
                        "按 1 / 2 / 3 选择一项强化",
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
                            for (i, opt) in choices.options.iter().enumerate() {
                                spawn_augment_card(row, &assets, i, opt);
                            }
                        });
                });
        });
}

fn spawn_augment_card(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    index: usize,
    opt: &AugmentChoiceOption,
) {
    let border_color = rarity_color(opt.rarity);
    let upgrade_tag = if opt.is_upgrade { " ★升级" } else { "" };

    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(220.0),
                    min_height: Val::Px(160.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.08, 0.08, 0.14, 0.95)),
                border_color: BorderColor(border_color),
                ..default()
            },
            AugmentButton { index },
        ))
        .with_children(|card| {
            // Key hint
            card.spawn(TextBundle::from_section(
                format!("[{}]", index + 1),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 14.0,
                    color: Color::srgb(0.6, 0.6, 0.7),
                },
            ));
            // Rarity + upgrade tag
            card.spawn(TextBundle::from_section(
                format!("{}{}", rarity_label(opt.rarity), upgrade_tag),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 13.0,
                    color: border_color,
                },
            ));
            // Title
            card.spawn(TextBundle::from_section(
                &opt.title,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ));
            // Description
            card.spawn(TextBundle::from_section(
                &opt.description,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 14.0,
                    color: Color::srgb(0.78, 0.80, 0.86),
                },
            ));
        });
}

pub fn augment_select_input(
    keys: Res<ButtonInput<KeyCode>>,
    choices: Res<AugmentChoices>,
    mut player_q: Query<&mut AugmentInventory, With<Player>>,
    mut next_state: ResMut<NextState<AppState>>,
    button_q: Query<(&Interaction, &AugmentButton), Changed<Interaction>>,
) {
    let mut picked: Option<usize> = None;

    // Keyboard: 1/2/3
    if keys.just_pressed(KeyCode::Digit1) || keys.just_pressed(KeyCode::Numpad1) {
        picked = Some(0);
    } else if keys.just_pressed(KeyCode::Digit2) || keys.just_pressed(KeyCode::Numpad2) {
        picked = Some(1);
    } else if keys.just_pressed(KeyCode::Digit3) || keys.just_pressed(KeyCode::Numpad3) {
        picked = Some(2);
    }

    // Mouse click
    for (interaction, btn) in &button_q {
        if *interaction == Interaction::Pressed {
            picked = Some(btn.index);
        }
    }

    let Some(index) = picked else { return };
    let Some(opt) = choices.options.get(index) else {
        return;
    };

    // Apply augment to player
    if let Ok(mut inventory) = player_q.get_single_mut() {
        inventory.add(opt.id);
    }

    let return_to = choices.return_state.unwrap_or(AppState::InGame);
    next_state.set(return_to);
}

pub fn cleanup_augment_select_ui(mut commands: Commands, q: Query<Entity, With<AugmentSelectUi>>) {
    for entity in &q {
        commands.entity(entity).despawn_recursive();
    }
}
