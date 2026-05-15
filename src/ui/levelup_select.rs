use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::data::definitions::RewardScalingConfig;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::player::components::*;
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::rewards::apply::heal_amount;
use crate::states::GamePhase;
use crate::ui::widgets;

/// One attribute option offered on level up.
#[derive(Debug, Clone)]
pub struct LevelUpOption {
    pub label: String,
    pub description: String,
    pub apply: LevelUpStat,
}

#[derive(Debug, Clone, Copy)]
pub enum LevelUpStat {
    AttackPower(f32),
    MaxHealth(f32),
    RecoverHealth(f32),
    MoveSpeed(f32),
    CritChance(f32),
    AttackSpeed(f32),
    DashCooldown(f32),
}

/// Resource: holds the level-up choices.
#[derive(Resource, Debug, Clone, Default)]
pub struct LevelUpChoices {
    pub options: Vec<LevelUpOption>,
    pub return_state: Option<GamePhase>,
    pub new_level: u32,
}

#[derive(Component)]
pub struct LevelUpSelectUi;

#[derive(Component)]
pub struct LevelUpButton {
    pub index: usize,
}

pub fn setup_levelup_ui(
    mut commands: Commands,
    assets: Res<GameAssets>,
    choices: Res<LevelUpChoices>,
    health_q: Query<&Health, With<Player>>,
    floor: Option<Res<FloorNumber>>,
    data: Option<Res<GameDataRegistry>>,
) {
    let max_health = health_q
        .get_single()
        .map(|health| health.max)
        .unwrap_or(100.0);
    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
    let default_scaling;
    let scaling = if let Some(data) = data.as_ref() {
        &data.rewards.scaling
    } else {
        default_scaling = RewardScalingConfig::default_config();
        &default_scaling
    };
    let heal_value = heal_amount(scaling, max_health, floor_number);

    commands
        .spawn((
            widgets::root_node(),
            LevelUpSelectUi,
            Name::new("LevelUpRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::panel_node(Color::srgba(0.02, 0.04, 0.02, 0.94)))
                .with_children(|panel| {
                    panel.spawn(widgets::title_text(
                        &assets,
                        &format!("升级！ Lv.{}", choices.new_level),
                        30.0,
                    ));
                    panel.spawn(widgets::title_text(
                        &assets,
                        "选择回血恢复状态，或选择一项属性提升",
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
                            row.spawn(widgets::panel_node(Color::srgba(0.12, 0.16, 0.12, 0.95)))
                                .with_children(|col| {
                                    col.spawn(widgets::title_text(&assets, "回血", 24.0));
                                    spawn_levelup_heal_card(col, &assets, heal_value);
                                });

                            row.spawn(widgets::panel_node(Color::srgba(0.12, 0.12, 0.18, 0.95)))
                                .with_children(|col| {
                                    col.spawn(widgets::title_text(&assets, "属性强化", 24.0));
                                    for (i, opt) in
                                        choices.options.iter().enumerate().skip(1).take(3)
                                    {
                                        spawn_levelup_card(col, &assets, i, opt);
                                    }
                                });
                        });
                });
        });
}

fn spawn_levelup_heal_card(parent: &mut ChildBuilder, assets: &GameAssets, heal_value: f32) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(250.0),
                    height: Val::Px(250.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.18, 0.32, 0.18)),
                border_color: BorderColor(Color::srgb(0.40, 0.75, 0.40)),
                ..default()
            },
            LevelUpButton { index: 0 },
        ))
        .with_children(|button| {
            button.spawn(widgets::title_text(assets, "1. 回血", 24.0));
            button.spawn(widgets::title_text(
                assets,
                format!("恢复 {:.0} 生命", heal_value),
                22.0,
            ));
            button.spawn(widgets::body_text(assets, "稳住当前状态后继续推进", 15.0));
        });
}

fn spawn_levelup_card(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    index: usize,
    opt: &LevelUpOption,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(320.0),
                    min_height: Val::Px(84.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.08, 0.12, 0.08, 0.95)),
                border_color: BorderColor(Color::srgb(0.40, 0.75, 0.40)),
                ..default()
            },
            LevelUpButton { index },
        ))
        .with_children(|card| {
            card.spawn(TextBundle::from_section(
                format!("[{}]", index + 1),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 14.0,
                    color: Color::srgb(0.6, 0.7, 0.6),
                },
            ));
            card.spawn(TextBundle::from_section(
                &opt.label,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 20.0,
                    color: Color::srgb(0.40, 0.90, 0.40),
                },
            ));
            card.spawn(TextBundle::from_section(
                &opt.description,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 14.0,
                    color: Color::srgb(0.78, 0.82, 0.78),
                },
            ));
        });
}

pub fn levelup_input(
    keys: Res<ButtonInput<KeyCode>>,
    choices: Res<LevelUpChoices>,
    mut player_q: Query<
        (
            &mut Health,
            &mut AttackPower,
            &mut MoveSpeed,
            &mut CritChance,
            &mut AttackCooldown,
            &mut DashCooldown,
        ),
        With<Player>,
    >,
    mut next_state: ResMut<NextState<GamePhase>>,
    button_q: Query<(&Interaction, &LevelUpButton), Changed<Interaction>>,
) {
    let mut picked: Option<usize> = None;

    if keys.just_pressed(KeyCode::Digit1) || keys.just_pressed(KeyCode::Numpad1) {
        picked = Some(0);
    } else if keys.just_pressed(KeyCode::Digit2) || keys.just_pressed(KeyCode::Numpad2) {
        picked = Some(1);
    } else if keys.just_pressed(KeyCode::Digit3) || keys.just_pressed(KeyCode::Numpad3) {
        picked = Some(2);
    } else if keys.just_pressed(KeyCode::Digit4) || keys.just_pressed(KeyCode::Numpad4) {
        picked = Some(3);
    }

    for (interaction, btn) in &button_q {
        if *interaction == Interaction::Pressed {
            picked = Some(btn.index);
        }
    }

    let Some(index) = picked else { return };
    let Some(opt) = choices.options.get(index) else {
        return;
    };

    if let Ok((mut health, mut atk, mut spd, mut crit, mut atk_cd, mut dash_cd)) =
        player_q.get_single_mut()
    {
        match opt.apply {
            LevelUpStat::AttackPower(v) => atk.0 += v,
            LevelUpStat::MaxHealth(v) => {
                health.max += v;
                health.current += v;
            }
            LevelUpStat::RecoverHealth(amount) => {
                health.current = (health.current + amount).min(health.max);
            }
            LevelUpStat::MoveSpeed(v) => spd.0 += v,
            LevelUpStat::CritChance(v) => crit.0 = (crit.0 + v).min(0.80),
            LevelUpStat::AttackSpeed(v) => {
                let new_dur = (atk_cd.timer.duration().as_secs_f32() - v).max(0.15);
                atk_cd
                    .timer
                    .set_duration(std::time::Duration::from_secs_f32(new_dur));
            }
            LevelUpStat::DashCooldown(v) => {
                let new_dur = (dash_cd.timer.duration().as_secs_f32() - v).max(0.3);
                dash_cd
                    .timer
                    .set_duration(std::time::Duration::from_secs_f32(new_dur));
            }
        }
    }

    let return_to = choices.return_state.unwrap_or(GamePhase::Playing);
    next_state.set(return_to);
}

pub fn cleanup_levelup_ui(mut commands: Commands, q: Query<Entity, With<LevelUpSelectUi>>) {
    for entity in &q {
        commands.entity(entity).despawn_recursive();
    }
}
