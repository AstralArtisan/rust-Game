use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::gameplay::shop::{ShopLine, ShopOffers, ShopUiLine, next_refresh_cost};
use crate::states::AppState;
use crate::ui::widgets;

#[derive(Component)]
pub struct ShopUi;

#[derive(Component)]
pub struct ShopLines;

pub fn setup_shop_ui(mut commands: Commands, assets: Res<GameAssets>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                ..default()
            },
            ShopUi,
            Name::new("ShopUiRoot"),
        ))
        .with_children(|root| {
            root.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(620.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        row_gap: Val::Px(12.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.10, 0.10, 0.12)),
                    ..default()
                },
                Name::new("ShopPanel"),
            ))
            .with_children(|panel| {
                panel.spawn(widgets::title_text(&assets, "商店", 28.0));
                panel.spawn(widgets::body_text(
                    &assets,
                    "1/2/3 属性 | 4/5/6 强化 | 7/8 工具 | R 刷新 | Esc 关闭",
                    18.0,
                ));
                panel.spawn((
                    NodeBundle {
                        style: Style {
                            row_gap: Val::Px(8.0),
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        ..default()
                    },
                    ShopLines,
                    Name::new("ShopLines"),
                ));
            });
        });
}

pub fn update_shop_ui(
    offers: Res<ShopOffers>,
    assets: Res<GameAssets>,
    mut commands: Commands,
    q: Query<Entity, With<ShopLines>>,
    existing: Query<Entity, With<ShopUiLine>>,
) {
    if !offers.is_changed() && existing.iter().next().is_some() {
        return;
    }
    for e in &existing {
        commands.entity(e).despawn_recursive();
    }
    let Ok(root) = q.get_single() else { return };
    commands.entity(root).with_children(|lines| {
        let refresh_cost = next_refresh_cost(offers.refresh_count);
        let refresh_text = if refresh_cost == 0 {
            "刷新：本次免费".to_string()
        } else {
            format!("刷新：{} 金币", refresh_cost)
        };
        lines.spawn((widgets::body_text(&assets, refresh_text, 18.0), ShopUiLine));
        spawn_shop_section(lines, &assets, "属性", &offers.lines, 1);
        spawn_shop_section(lines, &assets, "强化", &offers.augment_lines, 4);
        spawn_shop_section(lines, &assets, "工具", &offers.utility_lines, 7);
    });
}

fn spawn_shop_section(
    lines: &mut ChildBuilder,
    assets: &GameAssets,
    title: &str,
    section_lines: &[ShopLine],
    key_start: usize,
) {
    lines.spawn((widgets::title_text(assets, title, 22.0), ShopUiLine));
    for (i, line) in section_lines.iter().enumerate() {
        let key = key_start + i;
        lines.spawn((
            widgets::body_text(
                assets,
                if line.purchased {
                    format!("{}）{}（已购买）", key, line.title)
                } else {
                    format!("{}）{}（价格：{}）", key, line.title, line.cost)
                },
                20.0,
            ),
            ShopUiLine,
        ));
        lines.spawn((
            widgets::body_text(assets, line.description.clone(), 16.0),
            ShopUiLine,
        ));
    }
}

pub fn shop_ui_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next.set(AppState::InGame);
    }
}

pub fn cleanup_shop_ui(mut commands: Commands, q: Query<Entity, With<ShopUi>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}
