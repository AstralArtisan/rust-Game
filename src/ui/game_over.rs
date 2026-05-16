use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::states::AppState;
use crate::ui::widgets;

#[derive(Component)]
pub struct EndScreenUi;

#[derive(Component)]
pub struct EndScreenReturnButton;

pub fn setup_game_over_screen(mut commands: Commands, assets: Res<GameAssets>) {
    setup_end_screen(
        &mut commands,
        &assets,
        "你已倒下",
        "按 Enter 或点击按钮返回主菜单",
    );
}

pub fn setup_victory_screen(mut commands: Commands, assets: Res<GameAssets>) {
    setup_end_screen(
        &mut commands,
        &assets,
        "通关成功",
        "按 Enter 或点击按钮返回主菜单",
    );
}

fn setup_end_screen(commands: &mut Commands, assets: &GameAssets, title: &str, hint: &str) {
    commands
        .spawn((
            widgets::overlay_root_node(),
            EndScreenUi,
            Name::new("EndScreenRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::adventure_panel_node(620.0))
                .with_children(|panel| {
                    panel.spawn(widgets::title_text(assets, title, 56.0));
                    panel.spawn(widgets::muted_text(assets, hint, 16.0));
                    panel
                        .spawn((widgets::button_bundle(), EndScreenReturnButton))
                        .with_children(|button| {
                            button.spawn(widgets::title_text(assets, "返回主菜单", 20.0));
                        });
                });
        });
}

pub fn end_screen_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    q: Query<Entity, With<EndScreenUi>>,
    mut button_q: Query<&Interaction, (Changed<Interaction>, With<EndScreenReturnButton>)>,
) {
    let clicked = button_q
        .iter_mut()
        .any(|interaction| *interaction == Interaction::Pressed);
    if keyboard.just_pressed(KeyCode::Enter) || clicked {
        next_state.set(AppState::MainMenu);
        for entity in &q {
            commands.entity(entity).despawn_recursive();
        }
    }
}
