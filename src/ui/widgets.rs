use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::gameplay::augment::data::AugmentRarity;

pub fn screen_bg_color() -> Color {
    Color::srgba(0.015, 0.018, 0.028, 0.72)
}

pub fn panel_color() -> Color {
    Color::srgba(0.035, 0.043, 0.064, 0.94)
}

pub fn modal_scrim_color(alpha: f32) -> Color {
    Color::srgba(0.0, 0.0, 0.0, alpha)
}

pub fn section_color() -> Color {
    Color::srgba(0.075, 0.090, 0.128, 0.95)
}

pub fn section_alt_color() -> Color {
    Color::srgba(0.095, 0.110, 0.160, 0.95)
}

pub fn info_color() -> Color {
    Color::srgba(0.055, 0.070, 0.108, 0.94)
}

pub fn input_color() -> Color {
    Color::srgba(0.055, 0.070, 0.100, 0.96)
}

pub fn button_base_color() -> Color {
    Color::srgb(0.145, 0.180, 0.250)
}

pub fn button_hover_color() -> Color {
    Color::srgb(0.205, 0.250, 0.350)
}

pub fn button_selected_color() -> Color {
    Color::srgb(0.245, 0.470, 0.300)
}

pub fn button_disabled_color() -> Color {
    Color::srgb(0.085, 0.090, 0.105)
}

pub fn button_danger_color() -> Color {
    Color::srgb(0.365, 0.145, 0.165)
}

pub fn button_danger_hover_color() -> Color {
    Color::srgb(0.52, 0.24, 0.24)
}

pub fn button_info_color() -> Color {
    Color::srgb(0.110, 0.220, 0.340)
}

pub fn button_info_hover_color() -> Color {
    Color::srgb(0.18, 0.31, 0.44)
}

pub fn sanctuary_color() -> Color {
    Color::srgb(0.27, 0.69, 0.54)
}

pub fn shop_augment_color() -> Color {
    Color::srgb(0.36, 0.42, 0.84)
}

pub fn shop_utility_color() -> Color {
    Color::srgb(0.27, 0.69, 0.48)
}

pub fn responsive_panel_node(width_vw: f32, max_height_vh: f32) -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Vw(width_vw),
            max_width: Val::Px(1400.0),
            max_height: Val::Vh(max_height_vh),
            padding: UiRect::all(Val::Px(18.0)),
            row_gap: Val::Px(12.0),
            column_gap: Val::Px(14.0),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Px(2.0)),
            overflow: Overflow::clip_y(),
            ..default()
        },
        background_color: BackgroundColor(panel_color()),
        border_color: BorderColor(Color::srgb(0.34, 0.42, 0.56)),
        ..default()
    }
}

#[allow(dead_code)]
pub fn scrollable_column() -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            overflow: Overflow::clip_y(),
            ..default()
        },
        ..default()
    }
}

pub fn chip_node(accent: Color) -> NodeBundle {
    NodeBundle {
        style: Style {
            padding: UiRect::axes(Val::Px(7.0), Val::Px(3.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        background_color: BackgroundColor(Color::srgba(0.11, 0.13, 0.22, 0.95)),
        border_color: BorderColor(accent),
        ..default()
    }
}

pub fn modal_overlay_node() -> NodeBundle {
    NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        background_color: BackgroundColor(modal_scrim_color(0.7)),
        z_index: ZIndex::Global(50),
        ..default()
    }
}

pub fn wrap_row_node(gap: f32) -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(gap),
            row_gap: Val::Px(gap),
            align_items: AlignItems::FlexStart,
            ..default()
        },
        ..default()
    }
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

pub fn overlay_root_node() -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        },
        background_color: BackgroundColor(screen_bg_color()),
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

pub fn adventure_panel_node(width: f32) -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Px(width),
            max_width: Val::Percent(96.0),
            max_height: Val::Percent(94.0),
            padding: UiRect::all(Val::Px(14.0)),
            row_gap: Val::Px(10.0),
            column_gap: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        background_color: BackgroundColor(panel_color()),
        border_color: BorderColor(Color::srgb(0.34, 0.42, 0.56)),
        ..default()
    }
}

pub fn card_node(width: f32, min_height: f32, accent: Color) -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Px(width),
            min_height: Val::Px(min_height),
            padding: UiRect::all(Val::Px(12.0)),
            row_gap: Val::Px(6.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        background_color: BackgroundColor(section_color()),
        border_color: BorderColor(accent),
        ..default()
    }
}

#[allow(dead_code)]
pub fn card_button(width: f32, min_height: f32, accent: Color) -> ButtonBundle {
    ButtonBundle {
        style: Style {
            width: Val::Px(width),
            min_height: Val::Px(min_height),
            padding: UiRect::all(Val::Px(14.0)),
            row_gap: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::FlexStart,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        background_color: BackgroundColor(button_base_color()),
        border_color: BorderColor(accent),
        ..default()
    }
}

pub fn content_row_node() -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            column_gap: Val::Px(12.0),
            align_items: AlignItems::Stretch,
            ..default()
        },
        ..default()
    }
}

pub fn summary_panel_node() -> NodeBundle {
    NodeBundle {
        style: Style {
            min_width: Val::Px(268.0),
            flex_grow: 1.0,
            min_height: Val::Px(360.0),
            padding: UiRect::all(Val::Px(10.0)),
            row_gap: Val::Px(5.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        background_color: BackgroundColor(info_color()),
        border_color: BorderColor(Color::srgb(0.30, 0.38, 0.52)),
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

#[allow(dead_code)]
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

pub fn accent_text(
    assets: &GameAssets,
    text: impl Into<String>,
    size: f32,
    color: Color,
) -> TextBundle {
    TextBundle::from_section(
        text,
        TextStyle {
            font: assets.font.clone(),
            font_size: size,
            color,
        },
    )
}

pub fn section_heading(assets: &GameAssets, text: impl Into<String>) -> TextBundle {
    accent_text(assets, text, 16.0, Color::srgb(0.88, 0.76, 0.48))
}

pub fn button_bundle() -> ButtonBundle {
    ButtonBundle {
        style: Style {
            width: Val::Px(240.0),
            height: Val::Px(42.0),
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

pub fn rarity_color(rarity: AugmentRarity) -> Color {
    match rarity {
        AugmentRarity::Common => Color::srgb(0.76, 0.80, 0.86),
        AugmentRarity::Elite => Color::srgb(0.38, 0.62, 1.00),
        AugmentRarity::Legendary => Color::srgb(1.00, 0.74, 0.26),
    }
}

pub fn rarity_label(rarity: AugmentRarity) -> &'static str {
    match rarity {
        AugmentRarity::Common => "普通",
        AugmentRarity::Elite => "精英",
        AugmentRarity::Legendary => "传说",
    }
}

pub fn hp_color() -> Color {
    Color::srgb(0.72, 0.20, 0.24)
}

pub fn energy_color() -> Color {
    Color::srgb(0.22, 0.66, 0.96)
}

pub fn gold_color() -> Color {
    Color::srgb(0.95, 0.74, 0.28)
}

pub fn skill_color() -> Color {
    Color::srgb(0.56, 0.42, 0.92)
}

pub fn apply_button_interaction(
    interaction: Interaction,
    color: &mut BackgroundColor,
    normal: Color,
) {
    color.0 = match interaction {
        Interaction::Pressed => button_selected_color(),
        Interaction::Hovered => button_hover_color(),
        Interaction::None => normal,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adventure_theme_has_distinct_rarity_colors() {
        assert_ne!(
            rarity_color(AugmentRarity::Common),
            rarity_color(AugmentRarity::Elite)
        );
        assert_ne!(
            rarity_color(AugmentRarity::Elite),
            rarity_color(AugmentRarity::Legendary)
        );
    }
}
