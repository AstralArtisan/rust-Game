use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::AugmentRarity;
use crate::gameplay::player::components::Player;
use crate::gameplay::rewards::systems::{
    RewardFlow, RewardFlowStep, RewardPendingAction, RewardRoomAugmentService, RewardUiAction,
};
use crate::ui::character_panel::{self, CharacterSummary, CharacterSummaryItem};
use crate::ui::tooltip::TooltipContent;
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

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RewardUiRenderKey {
    step: RewardUiStepKind,
    option_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RewardUiStepKind {
    Inactive,
    Sanctuary,
    ForgePick,
}

impl RewardUiRenderKey {
    fn from_step(step: &RewardFlowStep) -> Self {
        match step {
            RewardFlowStep::Inactive => Self {
                step: RewardUiStepKind::Inactive,
                option_count: 0,
            },
            RewardFlowStep::Sanctuary(draft) => {
                let option_count = match &draft.augment_service {
                    RewardRoomAugmentService::Forge(options) => options.len(),
                };
                Self {
                    step: RewardUiStepKind::Sanctuary,
                    option_count,
                }
            }
            RewardFlowStep::ForgePick(options) => Self {
                step: RewardUiStepKind::ForgePick,
                option_count: options.len(),
            },
        }
    }
}

pub fn setup_reward_ui(
    mut commands: Commands,
    assets: Res<GameAssets>,
    flow: Res<RewardFlow>,
    data: Option<Res<GameDataRegistry>>,
    summary_q: Query<CharacterSummaryItem<'_>, With<Player>>,
) {
    let summary = character_panel::character_summary_from_query(&summary_q, data.as_deref());
    let render_key = RewardUiRenderKey::from_step(&flow.step);
    spawn_reward_root(&mut commands, &assets, &flow, &summary);
    commands.insert_resource(render_key);
}

fn spawn_reward_root(
    commands: &mut Commands,
    assets: &GameAssets,
    flow: &RewardFlow,
    summary: &CharacterSummary,
) {
    commands
        .spawn((
            widgets::overlay_root_node(),
            RewardUi,
            Name::new("RewardRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::responsive_panel_node(76.0, 85.0))
                .with_children(|panel| match &flow.step {
                    RewardFlowStep::Inactive => {
                        panel.spawn(widgets::title_text(assets, "圣所", 26.0));
                        character_panel::spawn_character_summary(panel, assets, summary);
                    }
                    RewardFlowStep::Sanctuary(draft) => {
                        panel.spawn(widgets::title_text(assets, "圣所", 28.0));
                        panel.spawn(widgets::muted_text(
                            assets,
                            "选择一项整备服务，完成后继续前进",
                            14.0,
                        ));
                        panel
                            .spawn(widgets::content_row_node())
                            .with_children(|row| {
                                character_panel::spawn_character_summary(row, assets, summary);
                                row.spawn(NodeBundle {
                                    style: Style {
                                        flex_grow: 1.0,
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(12.0),
                                        align_items: AlignItems::Stretch,
                                        ..default()
                                    },
                                    ..default()
                                })
                                .with_children(|cards| {
                                    spawn_service_card(
                                        cards,
                                        assets,
                                        0,
                                        "疗愈",
                                        "回满生命与能量",
                                        "稳住状态，适合在高压后重整节奏。",
                                    );

                                    match &draft.augment_service {
                                        RewardRoomAugmentService::Forge(options) => {
                                            let preview = preview_titles(options);
                                            spawn_service_card(
                                                cards,
                                                assets,
                                                1,
                                                "锻造",
                                                format!(
                                                    "从 {} 个强化候选中选择 1 个",
                                                    options.len()
                                                ),
                                                if preview.is_empty() {
                                                    "当前没有可用锻造候选。".to_string()
                                                } else {
                                                    format!("候选：{preview}")
                                                },
                                            );
                                        }
                                    }

                                    spawn_service_card(
                                        cards,
                                        assets,
                                        2,
                                        "启示",
                                        "获得一次额外升级选择",
                                        "立刻提升 1 级，并进入升级选择。",
                                    );
                                });
                            });
                    }
                    RewardFlowStep::ForgePick(options) => {
                        panel.spawn(widgets::title_text(assets, "锻造", 28.0));
                        panel.spawn(widgets::muted_text(
                            assets,
                            "选择一个强化候选，Esc 返回圣所选项",
                            14.0,
                        ));
                        panel
                            .spawn(widgets::content_row_node())
                            .with_children(|row| {
                                character_panel::spawn_character_summary(row, assets, summary);
                                spawn_augment_choice_row(row, assets, options);
                            });
                        spawn_back_button(panel, assets);
                    }
                });
        });
}

pub fn update_reward_ui(
    mut commands: Commands,
    assets: Res<GameAssets>,
    flow: Res<RewardFlow>,
    data: Option<Res<GameDataRegistry>>,
    summary_q: Query<CharacterSummaryItem<'_>, With<Player>>,
    mut rendered: Option<ResMut<RewardUiRenderKey>>,
    q: Query<Entity, With<RewardUi>>,
) {
    let render_key = RewardUiRenderKey::from_step(&flow.step);
    if rendered.as_deref().is_some_and(|key| *key == render_key) {
        return;
    }

    for entity in &q {
        safe_despawn_recursive(&mut commands, entity);
    }
    let summary = character_panel::character_summary_from_query(&summary_q, data.as_deref());
    spawn_reward_root(&mut commands, &assets, &flow, &summary);

    if let Some(rendered) = rendered.as_mut() {
        **rendered = render_key;
    } else {
        commands.insert_resource(render_key);
    }
}

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
                color.0 = widgets::button_hover_color();
            }
            Interaction::None => {
                color.0 = if back_button.is_some() {
                    widgets::button_danger_color()
                } else {
                    widgets::button_base_color()
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
    commands.remove_resource::<RewardUiRenderKey>();
}

fn spawn_service_card(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    index: usize,
    title: impl Into<String>,
    subtitle: impl Into<String>,
    description: impl Into<String>,
) {
    let accent = match index {
        0 => widgets::sanctuary_color(),
        1 => widgets::shop_augment_color(),
        _ => widgets::gold_color(),
    };
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    flex_grow: 1.0,
                    min_width: Val::Px(180.0),
                    min_height: Val::Px(150.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::FlexStart,
                    row_gap: Val::Px(6.0),
                    border: UiRect::top(Val::Px(3.0)),
                    ..default()
                },
                background_color: BackgroundColor(widgets::button_base_color()),
                border_color: BorderColor(accent),
                ..default()
            },
            RewardActionButton { index },
        ))
        .with_children(|button| {
            button.spawn(widgets::accent_text(
                assets,
                format!("[{}] {}", index + 1, title.into()),
                18.0,
                accent,
            ));
            button.spawn(widgets::title_text(assets, subtitle.into(), 13.0));
            button.spawn(widgets::body_text(assets, description.into(), 12.0));
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
                column_gap: Val::Px(12.0),
                align_items: AlignItems::FlexStart,
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            for (index, option) in options.iter().enumerate() {
                row.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(220.0),
                            min_height: Val::Px(60.0),
                            padding: UiRect::all(Val::Px(10.0)),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::FlexStart,
                            row_gap: Val::Px(4.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(widgets::button_base_color()),
                        border_color: BorderColor(widgets::rarity_color(option.rarity)),
                        ..default()
                    },
                    RewardActionButton { index },
                    TooltipContent {
                        title: option.title.clone(),
                        rarity: Some(option.rarity),
                        body: option.description.clone(),
                        tradeoff: None,
                        price: None,
                    },
                ))
                .with_children(|button| {
                    button.spawn(widgets::accent_text(
                        assets,
                        format!("[{}] {}", index + 1, option.title),
                        16.0,
                        widgets::rarity_color(option.rarity),
                    ));
                    button.spawn(widgets::muted_text(
                        assets,
                        format!(
                            "{}{}",
                            rarity_label(option.rarity),
                            if option.is_upgrade {
                                " · 升级"
                            } else {
                                ""
                            }
                        ),
                        11.0,
                    ));
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
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
                background_color: BackgroundColor(widgets::button_danger_color()),
                ..default()
            },
            RewardBackButton,
        ))
        .with_children(|button| {
            button.spawn(widgets::title_text(&assets, "[Esc] 返回", 16.0));
        });
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
