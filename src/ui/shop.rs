use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::player::components::Player;
use crate::gameplay::shop::{
    ShopLine, ShopOffers, ShopPendingAction, ShopSection, ShopUiAction, ShopUiLine,
    next_refresh_cost,
};
use crate::states::GamePhase;
use crate::ui::character_panel::{self, CharacterSummaryItem};
use crate::ui::tooltip::TooltipContent;
use crate::ui::widgets;

#[derive(Component)]
pub struct ShopUi;

#[derive(Component)]
pub struct ShopAttrColumn;

#[derive(Component)]
pub struct ShopAugColumn;

#[derive(Component)]
pub struct ShopUtilColumn;

#[derive(Component, Debug, Clone, Copy)]
pub struct ShopActionButton {
    pub action: ShopUiAction,
}

pub fn setup_shop_ui(
    mut commands: Commands,
    assets: Res<GameAssets>,
    data: Option<Res<GameDataRegistry>>,
    summary_q: Query<CharacterSummaryItem<'_>, With<Player>>,
) {
    let summary = character_panel::character_summary_from_query(&summary_q, data.as_deref());
    commands
        .spawn((
            widgets::overlay_root_node(),
            ShopUi,
            Name::new("ShopUiRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::responsive_panel_node(82.0, 88.0))
                .with_children(|panel| {
                    panel.spawn(widgets::title_text(&assets, "商店", 28.0));
                    panel.spawn(widgets::muted_text(
                        &assets,
                        "鼠标悬停查看详情，数字键快捷购买，R 刷新，Esc 关闭",
                        12.0,
                    ));
                    panel
                        .spawn(widgets::content_row_node())
                        .with_children(|row| {
                            character_panel::spawn_character_summary(row, &assets, &summary);
                            row.spawn(NodeBundle {
                                style: Style {
                                    flex_grow: 1.0,
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(8.0),
                                    align_items: AlignItems::FlexStart,
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|cols| {
                                spawn_shop_column(
                                    cols,
                                    &assets,
                                    "属性区",
                                    widgets::gold_color(),
                                    ShopAttrColumn,
                                );
                                spawn_shop_column(
                                    cols,
                                    &assets,
                                    "强化区",
                                    widgets::shop_augment_color(),
                                    ShopAugColumn,
                                );
                                spawn_shop_column(
                                    cols,
                                    &assets,
                                    "工具区",
                                    widgets::shop_utility_color(),
                                    ShopUtilColumn,
                                );
                            });
                        });
                });
        });
}

fn spawn_shop_column(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    title: &str,
    accent: Color,
    marker: impl Component,
) {
    parent
        .spawn((
            NodeBundle {
                style: Style {
                    flex_grow: 1.0,
                    min_height: Val::Px(300.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    align_items: AlignItems::Stretch,
                    border: UiRect::top(Val::Px(3.0)),
                    ..default()
                },
                background_color: BackgroundColor(widgets::section_color()),
                border_color: BorderColor(accent),
                ..default()
            },
            marker,
        ))
        .with_children(|col| {
            col.spawn(widgets::accent_text(assets, title, 13.0, accent));
        });
}

pub fn update_shop_ui(
    offers: Res<ShopOffers>,
    assets: Res<GameAssets>,
    data: Option<Res<GameDataRegistry>>,
    mut commands: Commands,
    attr_q: Query<Entity, With<ShopAttrColumn>>,
    aug_q: Query<Entity, With<ShopAugColumn>>,
    util_q: Query<Entity, With<ShopUtilColumn>>,
    existing: Query<Entity, With<ShopUiLine>>,
) {
    if !offers.is_changed() && existing.iter().next().is_some() {
        return;
    }
    for e in &existing {
        commands.entity(e).despawn_recursive();
    }

    if let Ok(root) = attr_q.get_single() {
        commands.entity(root).with_children(|col| {
            spawn_shop_items(
                col,
                &assets,
                ShopSection::Attributes,
                &offers.lines,
                &["1", "2", "3", "4"],
            );
        });
    }
    if let Ok(root) = aug_q.get_single() {
        commands.entity(root).with_children(|col| {
            spawn_shop_items(
                col,
                &assets,
                ShopSection::Augments,
                &offers.augment_lines,
                &["5", "6", "7", "8", "9"],
            );
        });
    }
    if let Ok(root) = util_q.get_single() {
        commands.entity(root).with_children(|col| {
            spawn_shop_items(
                col,
                &assets,
                ShopSection::Utilities,
                &offers.utility_lines,
                &["0", "-", "="],
            );
            let shop_cfg = data.as_deref().map(|d| d.shop.clone()).unwrap_or_default();
            let refresh_cost = next_refresh_cost(offers.refresh_count, &shop_cfg);
            let refresh_text = if refresh_cost == 0 {
                "R 刷新（免费）".to_string()
            } else {
                format!("R 刷新（{}金）", refresh_cost)
            };
            spawn_shop_command_button(
                col,
                &assets,
                refresh_text,
                ShopUiAction::Refresh,
                widgets::button_info_color(),
            );
            spawn_shop_command_button(
                col,
                &assets,
                "关闭商店",
                ShopUiAction::Exit,
                widgets::button_danger_color(),
            );
        });
    }
}

fn spawn_shop_items(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    section: ShopSection,
    lines: &[ShopLine],
    keys: &[&str],
) {
    for (i, line) in lines.iter().enumerate() {
        let key = keys.get(i).copied().unwrap_or("?");
        let label = if line.purchased {
            format!("{key}) {} (已购)", line.title)
        } else {
            format!("{key}) {} · {}金", line.title, line.cost)
        };
        let color = if line.purchased {
            widgets::button_disabled_color()
        } else {
            widgets::button_base_color()
        };
        parent
            .spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(36.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        ..default()
                    },
                    background_color: BackgroundColor(color),
                    ..default()
                },
                ShopActionButton {
                    action: ShopUiAction::Select(section, i),
                },
                ShopUiLine,
                TooltipContent {
                    title: line.title.clone(),
                    rarity: None,
                    body: line.description.clone(),
                    tradeoff: None,
                    price: if line.purchased {
                        Some("已购买".to_string())
                    } else {
                        Some(format!("价格：{} 金币", line.cost))
                    },
                },
            ))
            .with_children(|button| {
                button.spawn(widgets::body_text(assets, &label, 13.0));
            });
    }
}

pub fn shop_ui_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut interaction_q: Query<
        (&Interaction, &ShopActionButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut pending_action: ResMut<ShopPendingAction>,
    mut next: ResMut<NextState<GamePhase>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next.set(GamePhase::Playing);
    }

    for (interaction, button, mut color) in &mut interaction_q {
        match *interaction {
            Interaction::Hovered => {
                color.0 = widgets::button_hover_color();
            }
            Interaction::None => {
                color.0 = match button.action {
                    ShopUiAction::Exit => widgets::button_danger_color(),
                    ShopUiAction::Refresh => widgets::button_info_color(),
                    ShopUiAction::Select(_, _) => widgets::button_base_color(),
                };
            }
            Interaction::Pressed => {
                pending_action.0 = Some(button.action);
            }
        }
    }
}

fn spawn_shop_command_button(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    label: impl Into<String>,
    action: ShopUiAction,
    color: Color,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(38.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(color),
                ..default()
            },
            ShopActionButton { action },
            ShopUiLine,
        ))
        .with_children(|button| {
            button.spawn(widgets::title_text(assets, label, 16.0));
        });
}

pub fn cleanup_shop_ui(mut commands: Commands, q: Query<Entity, With<ShopUi>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}
