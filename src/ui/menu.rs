use bevy::app::AppExit;
use bevy::prelude::*;
use std::path::Path;

use crate::core::achievements::{AchievementId, Achievements};
use crate::core::assets::GameAssets;
use crate::core::test_mode::TestMode;
use crate::gameplay::enemy::systems::EnemySpawnCount;
use crate::gameplay::progression::floor::FloorNumber;
use crate::states::AppState;
use crate::ui::widgets;

#[derive(Component)]
pub struct MainMenuUi;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainMenuScreen(pub MainMenuPage);

impl Default for MainMenuScreen {
    fn default() -> Self {
        Self(MainMenuPage::Home)
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainMenuRenderKey(MainMenuPage);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuPage {
    Home,
    Achievements,
    Codex,
    Settings,
    Saves,
}

#[derive(Component)]
pub(crate) enum MainMenuButton {
    SinglePlayer,
    TestMode,
    Multiplayer,
    Page(MainMenuPage),
    Quit,
}

pub fn setup_main_menu(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut screen: ResMut<MainMenuScreen>,
    _achievements: Option<Res<Achievements>>,
) {
    screen.0 = MainMenuPage::Home;

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ..default()
            },
            MainMenuUi,
            Name::new("MainMenuBg"),
        ))
        .with_children(|bg| {
            bg.spawn(ImageBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                image: UiImage::new(assets.textures.menu_background.clone()),
                ..default()
            });
        });

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    padding: UiRect::top(Val::Vh(12.0)),
                    row_gap: Val::Vh(4.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ..default()
            },
            MainMenuUi,
            Name::new("MainMenuRoot"),
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|title_group| {
                title_group.spawn(widgets::accent_text(
                    &assets,
                    "勇闯方块城",
                    72.0,
                    widgets::gold_color(),
                ));
                title_group.spawn(widgets::muted_text(&assets, "Block City Adventure", 16.0));
            });

            root.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    margin: UiRect::top(Val::Vh(4.0)),
                    ..default()
                },
                ..default()
            })
            .with_children(|menu| {
                spawn_menu_button(menu, &assets, "开始冒险", MainMenuButton::SinglePlayer);
                spawn_menu_button(menu, &assets, "测试模式", MainMenuButton::TestMode);
                spawn_menu_button(menu, &assets, "联机游戏", MainMenuButton::Multiplayer);
                spawn_menu_button(
                    menu,
                    &assets,
                    "成就",
                    MainMenuButton::Page(MainMenuPage::Achievements),
                );
                spawn_menu_button(
                    menu,
                    &assets,
                    "图鉴",
                    MainMenuButton::Page(MainMenuPage::Codex),
                );
                spawn_menu_button(
                    menu,
                    &assets,
                    "设置",
                    MainMenuButton::Page(MainMenuPage::Settings),
                );
                spawn_menu_button(
                    menu,
                    &assets,
                    "存档 / 读档",
                    MainMenuButton::Page(MainMenuPage::Saves),
                );
                spawn_menu_button(menu, &assets, "退出", MainMenuButton::Quit);
            });
        });
    commands.insert_resource(MainMenuRenderKey(screen.0));
}

pub fn menu_button_system(
    mut interaction_q: Query<
        (&Interaction, &MainMenuButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
    mut commands: Commands,
    mut screen: ResMut<MainMenuScreen>,
) {
    for (interaction, action, mut color) in &mut interaction_q {
        match *interaction {
            Interaction::Hovered => color.0 = widgets::button_hover_color(),
            Interaction::None => color.0 = widgets::button_base_color(),
            Interaction::Pressed => match action {
                MainMenuButton::SinglePlayer => {
                    commands.insert_resource(FloorNumber(1));
                    commands.insert_resource(EnemySpawnCount { current: 0 });
                    commands.insert_resource(TestMode(false));
                    next_state.set(AppState::InGame);
                }
                MainMenuButton::TestMode => {
                    commands.insert_resource(FloorNumber(1));
                    commands.insert_resource(EnemySpawnCount { current: 0 });
                    commands.insert_resource(TestMode(true));
                    next_state.set(AppState::InGame);
                }
                MainMenuButton::Multiplayer => {
                    next_state.set(AppState::MultiplayerMenu);
                }
                MainMenuButton::Page(page) => {
                    screen.0 = *page;
                }
                MainMenuButton::Quit => {
                    let _ = exit.send(AppExit::Success);
                }
            },
        }
    }
}

#[derive(Component)]
pub struct MainMenuModal;

pub fn update_main_menu_content(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut screen: ResMut<MainMenuScreen>,
    achievements: Option<Res<Achievements>>,
    mut rendered: Option<ResMut<MainMenuRenderKey>>,
    modal_q: Query<Entity, With<MainMenuModal>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) && screen.0 != MainMenuPage::Home {
        screen.0 = MainMenuPage::Home;
        return;
    }

    if rendered.as_deref().is_some_and(|key| key.0 == screen.0) {
        return;
    }

    for entity in &modal_q {
        commands.entity(entity).despawn_recursive();
    }

    if screen.0 != MainMenuPage::Home {
        commands
            .spawn((
                widgets::modal_overlay_node(),
                MainMenuModal,
                Name::new("MainMenuModal"),
            ))
            .with_children(|overlay| {
                overlay
                    .spawn(widgets::responsive_panel_node(60.0, 70.0))
                    .with_children(|panel| {
                        spawn_main_menu_page(panel, &assets, screen.0, achievements.as_deref());
                    });
            });
    }

    if let Some(rendered) = rendered.as_mut() {
        **rendered = MainMenuRenderKey(screen.0);
    } else {
        commands.insert_resource(MainMenuRenderKey(screen.0));
    }
}

pub fn cleanup_main_menu(
    mut commands: Commands,
    q: Query<Entity, With<MainMenuUi>>,
    modal_q: Query<Entity, With<MainMenuModal>>,
) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
    for e in &modal_q {
        commands.entity(e).despawn_recursive();
    }
    commands.remove_resource::<MainMenuRenderKey>();
}

fn spawn_menu_button(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    label: &str,
    action: MainMenuButton,
) {
    parent
        .spawn((widgets::button_bundle(), action))
        .with_children(|button| {
            button.spawn(widgets::title_text(assets, label, 18.0));
        });
}

fn spawn_main_menu_page(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    page: MainMenuPage,
    achievements: Option<&Achievements>,
) {
    match page {
        MainMenuPage::Home => spawn_home_page(parent, assets),
        MainMenuPage::Achievements => spawn_achievements_page(parent, assets, achievements),
        MainMenuPage::Codex => spawn_codex_page(parent, assets),
        MainMenuPage::Settings => spawn_settings_page(parent, assets),
        MainMenuPage::Saves => spawn_saves_page(parent, assets),
    }
}

fn spawn_home_page(parent: &mut ChildBuilder, assets: &GameAssets) {
    parent.spawn(widgets::section_heading(assets, "冒险状态"));
    parent.spawn(widgets::title_text(assets, "方块城的裂隙仍在扩张", 24.0));
    parent.spawn(widgets::body_text(
        assets,
        "冒险者，方块城深处传来异动。准备好你的武器，踏入裂隙吧。",
        15.0,
    ));
    parent.spawn(widgets::section_heading(assets, "操作"));
    for line in [
        "鼠标左键：近战    鼠标右键：远程",
        "Space：冲刺    E：交互    ESC：暂停",
        "数字键为快捷键，所有选择界面都支持鼠标点击。",
    ] {
        parent.spawn(widgets::muted_text(assets, line, 13.0));
    }
}

fn spawn_achievements_page(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    achievements: Option<&Achievements>,
) {
    parent.spawn(widgets::section_heading(assets, "成就"));
    let unlocked = achievements.map(|value| &value.unlocked);
    let labels = achievement_labels();
    let completed = labels
        .iter()
        .filter(|(id, _)| unlocked.map(|set| set.contains(id)).unwrap_or(false))
        .count();

    parent.spawn(widgets::muted_text(
        assets,
        format!("{completed}/{} 已完成", labels.len()),
        13.0,
    ));

    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(12.0),
                align_items: AlignItems::FlexStart,
                ..default()
            },
            ..default()
        })
        .with_children(|grid| {
            for (id, title) in labels {
                let achieved = unlocked.map(|set| set.contains(&id)).unwrap_or(false);
                spawn_achievement_card(grid, assets, id, title, achieved);
            }
        });
}

fn spawn_achievement_card(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    id: AchievementId,
    title: &str,
    achieved: bool,
) {
    let border = if achieved {
        widgets::gold_color()
    } else {
        Color::srgb(0.25, 0.29, 0.36)
    };
    let background = if achieved {
        Color::srgba(0.12, 0.11, 0.08, 0.94)
    } else {
        widgets::section_color()
    };
    let icon_tint = if achieved {
        Color::WHITE
    } else {
        Color::srgba(0.45, 0.46, 0.50, 0.55)
    };
    let status = if achieved { "已完成" } else { "未完成" };

    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(220.0),
                min_height: Val::Px(96.0),
                flex_grow: 1.0,
                flex_basis: Val::Px(220.0),
                padding: UiRect::all(Val::Px(10.0)),
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: BackgroundColor(background),
            border_color: BorderColor(border),
            ..default()
        })
        .with_children(|card| {
            if let Some(icon) = assets.textures.achievement_icons.get(&id) {
                card.spawn(ImageBundle {
                    style: Style {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    image: UiImage::new(icon.clone()).with_color(icon_tint),
                    ..default()
                });
            }

            card.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    flex_grow: 1.0,
                    ..default()
                },
                ..default()
            })
            .with_children(|text| {
                if achieved {
                    text.spawn(widgets::accent_text(
                        assets,
                        title,
                        15.0,
                        widgets::gold_color(),
                    ));
                } else {
                    text.spawn(widgets::muted_text(assets, title, 15.0));
                }
                text.spawn(widgets::muted_text(assets, status, 12.0));
            });
        });
}

fn spawn_codex_page(parent: &mut ChildBuilder, assets: &GameAssets) {
    parent.spawn(widgets::section_heading(assets, "图鉴"));
    for line in [
        "怪物、Boss、强化、终结技会在后续图鉴数据接入后逐项展示。",
        "局内角色面板会显示实时强化与终结技状态。",
    ] {
        parent.spawn(widgets::body_text(assets, line, 14.0));
    }
}

fn spawn_settings_page(parent: &mut ChildBuilder, assets: &GameAssets) {
    parent.spawn(widgets::section_heading(assets, "设置"));
    for line in [
        "当前设置页展示可用操作。",
        "音频、按键重绑、画面缩放将在后续接入。",
    ] {
        parent.spawn(widgets::body_text(assets, line, 14.0));
    }
}

fn spawn_saves_page(parent: &mut ChildBuilder, assets: &GameAssets) {
    parent.spawn(widgets::section_heading(assets, "存档 / 读档"));
    let save_exists = Path::new("saves/run_save.ron").exists();
    let status = if save_exists {
        "检测到本地存档。"
    } else {
        "未检测到本地存档。"
    };
    parent.spawn(widgets::body_text(assets, status, 14.0));
    parent.spawn(widgets::muted_text(
        assets,
        "游戏内 F5 保存 / F9 读取，暂停菜单也可操作。",
        13.0,
    ));
}

fn achievement_labels() -> [(AchievementId, &'static str); 9] {
    [
        (AchievementId::FirstBlood, "初战告捷"),
        (AchievementId::EliteSlayer, "精英猎手"),
        (AchievementId::Combo10, "连击十段"),
        (AchievementId::Rich, "钱袋鼓胀"),
        (AchievementId::Shopper, "第一次购物"),
        (AchievementId::PuzzleSolver, "事件破解者"),
        (AchievementId::BossSlayer, "Boss 讨伐"),
        (AchievementId::Untouchable, "无伤清房"),
        (AchievementId::Victory, "通关方块城"),
    ]
}
