use bevy::prelude::*;

use crate::core::assets::GameAssets;

pub fn panel_color() -> Color {
    Color::srgba(0.05, 0.06, 0.10, 0.92)
}

pub fn modal_scrim_color(alpha: f32) -> Color {
    Color::srgba(0.0, 0.0, 0.0, alpha)
}

pub fn section_color() -> Color {
    Color::srgba(0.10, 0.12, 0.18, 0.94)
}

pub fn section_alt_color() -> Color {
    Color::srgba(0.12, 0.14, 0.22, 0.95)
}

#[allow(dead_code)]
#[allow(dead_code)]
pub fn info_color() -> Color {
    Color::srgba(0.09, 0.11, 0.16, 0.92)
}

pub fn input_color() -> Color {
    Color::srgba(0.08, 0.10, 0.15, 0.95)
}

pub fn button_base_color() -> Color {
    Color::srgb(0.18, 0.22, 0.30)
}

pub fn button_hover_color() -> Color {
    Color::srgb(0.24, 0.28, 0.38)
}

pub fn button_selected_color() -> Color {
    Color::srgb(0.25, 0.52, 0.26)
}

pub fn button_disabled_color() -> Color {
    Color::srgb(0.12, 0.12, 0.14)
}

pub fn button_danger_color() -> Color {
    Color::srgb(0.42, 0.19, 0.19)
}

pub fn button_danger_hover_color() -> Color {
    Color::srgb(0.52, 0.24, 0.24)
}

pub fn button_info_color() -> Color {
    Color::srgb(0.14, 0.25, 0.36)
}

pub fn button_info_hover_color() -> Color {
    Color::srgb(0.18, 0.31, 0.44)
}

pub fn root_node() -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        background_color: BackgroundColor(Color::NONE),
        ..default()
    }
}

pub fn panel_node(color: Color) -> NodeBundle {
    NodeBundle {
        style: Style {
            padding: UiRect::all(Val::Px(18.0)),
            row_gap: Val::Px(10.0),
            column_gap: Val::Px(10.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        background_color: BackgroundColor(color),
        ..default()
    }
}

#[allow(dead_code)]
#[allow(dead_code)]
pub fn panel_node_with_padding(color: Color, padding: f32) -> NodeBundle {
    NodeBundle {
        style: Style {
            padding: UiRect::all(Val::Px(padding)),
            row_gap: Val::Px(10.0),
            column_gap: Val::Px(10.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        background_color: BackgroundColor(color),
        ..default()
    }
}

pub fn section_node(color: Color) -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(14.0)),
            row_gap: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        background_color: BackgroundColor(color),
        ..default()
    }
}

pub fn scrim_node(alpha: f32) -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        background_color: BackgroundColor(modal_scrim_color(alpha)),
        ..default()
    }
}

pub fn modal_panel_node(width: f32) -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Px(width),
            max_width: Val::Percent(92.0),
            padding: UiRect::all(Val::Px(18.0)),
            row_gap: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        background_color: BackgroundColor(panel_color()),
        ..default()
    }
}

pub fn input_field_node() -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
            ..default()
        },
        background_color: BackgroundColor(input_color()),
        ..default()
    }
}

pub fn title_text(assets: &GameAssets, text: impl Into<String>, size: f32) -> TextBundle {
    TextBundle::from_section(
        text,
        TextStyle {
            font: assets.font.clone(),
            font_size: size,
            color: Color::WHITE,
        },
    )
}

pub fn body_text(assets: &GameAssets, text: impl Into<String>, size: f32) -> TextBundle {
    TextBundle::from_section(
        text,
        TextStyle {
            font: assets.font.clone(),
            font_size: size,
            color: Color::srgb(0.92, 0.92, 0.95),
        },
    )
}

pub fn muted_text(assets: &GameAssets, text: impl Into<String>, size: f32) -> TextBundle {
    TextBundle::from_section(
        text,
        TextStyle {
            font: assets.font.clone(),
            font_size: size,
            color: Color::srgb(0.74, 0.78, 0.86),
        },
    )
}

pub fn button_bundle() -> ButtonBundle {
    ButtonBundle {
        style: Style {
            width: Val::Px(260.0),
            height: Val::Px(48.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        background_color: BackgroundColor(button_base_color()),
        ..default()
    }
}

pub fn button_bundle_sized(width: f32, height: f32) -> ButtonBundle {
    ButtonBundle {
        style: Style {
            width: Val::Px(width),
            min_height: Val::Px(height),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::all(Val::Px(12.0)),
            ..default()
        },
        background_color: BackgroundColor(button_base_color()),
        ..default()
    }
}
