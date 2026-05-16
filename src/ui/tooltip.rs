use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::core::assets::GameAssets;
use crate::gameplay::augment::data::AugmentRarity;
use crate::states::AppState;
use crate::ui::widgets;

#[derive(Component, Clone, Debug)]
pub struct TooltipContent {
    pub title: String,
    pub rarity: Option<AugmentRarity>,
    pub body: String,
    pub tradeoff: Option<String>,
    pub price: Option<String>,
}

#[derive(Component)]
pub struct TooltipPanel;

#[derive(Resource, Default)]
pub struct TooltipState {
    pub active_entity: Option<Entity>,
}

pub struct TooltipPlugin;

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TooltipState>().add_systems(
            Update,
            (
                tooltip_hover_system,
                tooltip_display_system.after(tooltip_hover_system),
                tooltip_position_system.after(tooltip_display_system),
            )
                .run_if(
                    in_state(AppState::InGame)
                        .or_else(in_state(AppState::CoopGame))
                        .or_else(in_state(AppState::MainMenu)),
                ),
        );
    }
}

fn tooltip_hover_system(
    interaction_q: Query<(Entity, &Interaction), (Changed<Interaction>, With<TooltipContent>)>,
    mut state: ResMut<TooltipState>,
) {
    for (entity, interaction) in &interaction_q {
        match *interaction {
            Interaction::Hovered => {
                state.active_entity = Some(entity);
            }
            Interaction::None => {
                if state.active_entity == Some(entity) {
                    state.active_entity = None;
                }
            }
            Interaction::Pressed => {}
        }
    }
}

fn tooltip_display_system(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut state: ResMut<TooltipState>,
    content_q: Query<&TooltipContent>,
    panel_q: Query<Entity, With<TooltipPanel>>,
) {
    if let Some(active) = state.active_entity {
        if content_q.get(active).is_err() {
            state.active_entity = None;
        }
    }

    if !state.is_changed() {
        return;
    }

    for entity in &panel_q {
        commands.entity(entity).despawn_recursive();
    }

    let Some(active) = state.active_entity else {
        return;
    };
    let Ok(content) = content_q.get(active) else {
        return;
    };

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    max_width: Val::Px(260.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.04, 0.05, 0.09, 0.96)),
                border_color: BorderColor(
                    content
                        .rarity
                        .map(widgets::rarity_color)
                        .unwrap_or(Color::srgb(0.3, 0.38, 0.52)),
                ),
                z_index: ZIndex::Global(200),
                ..default()
            },
            TooltipPanel,
        ))
        .with_children(|panel| {
            let title_color = content
                .rarity
                .map(widgets::rarity_color)
                .unwrap_or(Color::WHITE);
            panel.spawn(widgets::accent_text(&assets, &content.title, 14.0, title_color));

            if let Some(rarity) = content.rarity {
                panel.spawn(widgets::muted_text(
                    &assets,
                    widgets::rarity_label(rarity),
                    11.0,
                ));
            }

            panel.spawn(widgets::body_text(&assets, &content.body, 12.0));

            if let Some(tradeoff) = &content.tradeoff {
                panel.spawn(widgets::accent_text(
                    &assets,
                    tradeoff,
                    11.0,
                    Color::srgb(0.85, 0.45, 0.45),
                ));
            }

            if let Some(price) = &content.price {
                panel.spawn(widgets::accent_text(
                    &assets,
                    price,
                    11.0,
                    widgets::gold_color(),
                ));
            }
        });
}

fn tooltip_position_system(
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut panel_q: Query<&mut Style, With<TooltipPanel>>,
) {
    let Ok(window) = window_q.get_single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(mut style) = panel_q.get_single_mut() else {
        return;
    };

    let offset_x = 16.0;
    let offset_y = 16.0;
    let tooltip_width = 260.0;

    let left = if cursor.x + offset_x + tooltip_width > window.width() {
        (cursor.x - tooltip_width - offset_x).max(0.0)
    } else {
        cursor.x + offset_x
    };
    let top = (cursor.y + offset_y).min(window.height() - 200.0).max(0.0);

    style.left = Val::Px(left);
    style.top = Val::Px(top);
}
