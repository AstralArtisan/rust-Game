use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::gameplay::augment::data::AugmentRarity;
use crate::gameplay::rewards::systems::{
    RewardFlow, RewardFlowStep, RewardPendingAction, RewardRoomAugmentService, RewardUiAction,
};
use crate::ui::widgets;
use crate::utils::entity::safe_despawn_recursive;

#[derive(Component)]
pub struct RewardUi;

#[derive(Component, Debug, Clone, Copy)]
pub struct RewardActionButton {
    pub index: usize,
}

#[derive(Component)]
pub struct RewardBackButton;

pub fn setup_reward_ui(
    mut commands: Commands,
    assets: Res<GameAssets>,
    flow: Res<RewardFlow>,
) {
    commands
        .spawn((widgets::root_node(), RewardUi, Name::new("RewardRoot")))
        .with_children(|root| {
            root.spawn(widgets::panel_node(Color::srgba(0.04, 0.04, 0.06, 0.94)))
                .with_children(|panel| match &flow.step {
                    RewardFlowStep::Inactive => {
                        panel.spawn(widgets::title_text(&assets, "圣所", 30.0));
                    }
                    RewardFlowStep::Sanctuary(draft) => {
                        panel.spawn(widgets::title_text(&assets, "圣所", 30.0));
                        panel.spawn(widgets::title_text(
                            &assets,
                            "选择一项整备服务，完成后继续前进",
                            16.0,
                        ));
                        panel
                            .spawn(NodeBundle {
                                style: Style {
                                    column_gap: Val::Px(18.0),
                                    align_items: AlignItems::FlexStart,
                                    margin: UiRect::top(Val::Px(12.0)),
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|row| {
                                spawn_service_card(
                                    row,
                                    &assets,
                                    0,
                                    "疗愈",
                                    "回满生命与能量",
                                    "稳住状态，适合在高压后重整节奏。",
                                );

                                match &draft.augment_service {
                                    RewardRoomAugmentService::Upgrade(options) => {
                                        let preview = preview_titles(options);
                                        spawn_service_card(
                                            row,
                                            &assets,
                                            1,
                                            "淬炼",
                                            format!("从 {} 个可升级强化里选择 1 个", options.len()),
                                            if preview.is_empty() {
                                                "将一个 1 级强化提升到 2 级。".to_string()
                                            } else {
                                                format!("可升级：{preview}")
                                            },
                                        );
                                    }
                                    RewardRoomAugmentService::Awakening(options) => {
                                        let preview = preview_titles(options);
                                        spawn_service_card(
                                            row,
                                            &assets,
                                            1,
                                            "觉醒",
                                            "从 2 个精英/传说强化中选择 1 个",
                                            if preview.is_empty() {
                                                "当前没有可淬炼强化，改为提供高稀有强化。".to_string()
                                            } else {
                                                format!("候选：{preview}")
                                            },
                                        );
                                    }
                                }

                                spawn_service_card(
                                    row,
                                    &assets,
                                    2,
                                    "启示",
                                    "获得一次额外升级选择",
                                    "立刻提升 1 级，并进入升级选择。",
                                );
                            });
                    }
                    RewardFlowStep::UpgradePick(options) => {
                        panel.spawn(widgets::title_text(&assets, "淬炼", 30.0));
                        panel.spawn(widgets::title_text(
                            &assets,
                            "选择一个 1 级强化提升到 2 级，Esc 返回圣所选项",
                            16.0,
                        ));
                        spawn_augment_choice_row(panel, &assets, options);
                        spawn_back_button(panel, &assets);
                    }
                    RewardFlowStep::AwakeningPick(options) => {
                        panel.spawn(widgets::title_text(&assets, "觉醒", 30.0));
                        panel.spawn(widgets::title_text(
                            &assets,
                            "从高稀有强化中选择 1 个，Esc 返回圣所选项",
                            16.0,
                        ));
                        spawn_augment_choice_row(panel, &assets, options);
                        spawn_back_button(panel, &assets);
                    }
                });
        });
}

pub fn update_reward_ui() {}

pub fn reward_ui_input_system(
    mut interaction_q: Query<
        (
            &Interaction,
            Option<&RewardActionButton>,
            Option<&RewardBackButton>,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut pending_action: ResMut<RewardPendingAction>,
) {
    for (interaction, action_button, back_button, mut color) in &mut interaction_q {
        match *interaction {
            Interaction::Hovered => {
                color.0 = Color::srgb(0.28, 0.32, 0.44);
            }
            Interaction::None => {
                color.0 = if back_button.is_some() {
                    Color::srgb(0.28, 0.18, 0.18)
                } else {
                    Color::srgb(0.18, 0.22, 0.30)
                };
            }
            Interaction::Pressed => {
                if let Some(button) = action_button {
                    pending_action.0 = Some(RewardUiAction::Select(button.index));
                } else if back_button.is_some() {
                    pending_action.0 = Some(RewardUiAction::Back);
                }
            }
        }
    }
}

pub fn cleanup_reward_ui(mut commands: Commands, q: Query<Entity, With<RewardUi>>) {
    for entity in &q {
        safe_despawn_recursive(&mut commands, entity);
    }
}

fn spawn_service_card(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    index: usize,
    title: impl Into<String>,
    subtitle: impl Into<String>,
    description: impl Into<String>,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(240.0),
                    min_height: Val::Px(220.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::FlexStart,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.18, 0.22, 0.30)),
                ..default()
            },
            RewardActionButton { index },
        ))
        .with_children(|button| {
            button.spawn(widgets::title_text(
                assets,
                format!("[{}] {}", index + 1, title.into()),
                24.0,
            ));
            button.spawn(widgets::title_text(assets, subtitle.into(), 15.0));
            button.spawn(widgets::body_text(assets, description.into(), 14.0));
        });
}

fn spawn_augment_choice_row(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    options: &[crate::ui::augment_select::AugmentChoiceOption],
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                column_gap: Val::Px(18.0),
                align_items: AlignItems::FlexStart,
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            for (index, option) in options.iter().enumerate() {
                row.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(260.0),
                            min_height: Val::Px(220.0),
                            padding: UiRect::all(Val::Px(14.0)),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::FlexStart,
                            row_gap: Val::Px(8.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgb(0.18, 0.22, 0.30)),
                        border_color: BorderColor(rarity_color(option.rarity)),
                        ..default()
                    },
                    RewardActionButton { index },
                ))
                .with_children(|button| {
                    button.spawn(widgets::title_text(
                        assets,
                        format!("[{}] {}", index + 1, option.title),
                        22.0,
                    ));
                    button.spawn(widgets::body_text(
                        assets,
                        format!(
                            "{}{}",
                            rarity_label(option.rarity),
                            if option.is_upgrade { " · 升级后效果" } else { "" }
                        ),
                        14.0,
                    ));
                    button.spawn(widgets::body_text(assets, &option.description, 15.0));
                });
            }
        });
}

fn spawn_back_button(parent: &mut ChildBuilder, assets: &GameAssets) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(220.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(12.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.28, 0.18, 0.18)),
                ..default()
            },
            RewardBackButton,
        ))
        .with_children(|button| {
            button.spawn(widgets::title_text(&assets, "[Esc] 返回", 18.0));
        });
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

fn preview_titles(options: &[crate::ui::augment_select::AugmentChoiceOption]) -> String {
    options
        .iter()
        .map(|option| option.title.as_str())
        .collect::<Vec<_>>()
        .join(" / ")
}
