use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::gameplay::player::components::*;
use crate::states::AppState;
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
    MoveSpeed(f32),
    CritChance(f32),
    AttackSpeed(f32),
    DashCooldown(f32),
}

/// Resource: holds the level-up choices.
#[derive(Resource, Debug, Clone, Default)]
pub struct LevelUpChoices {
    pub options: Vec<LevelUpOption>,
    pub return_state: Option<AppState>,
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
) {
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
                    panel.spawn(widgets::title_text(&assets, "选择一项属性提升", 16.0));
                    panel
                        .spawn(NodeBundle {
                            style: Style {
                                column_gap: Val::Px(16.0),
                                align_items: AlignItems::FlexStart,
                                margin: UiRect::top(Val::Px(12.0)),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|row| {
                            for (i, opt) in choices.options.iter().enumerate() {
                                spawn_levelup_card(row, &assets, i, opt);
                            }
                        });
                });
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
                    width: Val::Px(200.0),
                    min_height: Val::Px(120.0),
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
    mut next_state: ResMut<NextState<AppState>>,
    button_q: Query<(&Interaction, &LevelUpButton), Changed<Interaction>>,
) {
    let mut picked: Option<usize> = None;

    if keys.just_pressed(KeyCode::Digit1) || keys.just_pressed(KeyCode::Numpad1) {
        picked = Some(0);
    } else if keys.just_pressed(KeyCode::Digit2) || keys.just_pressed(KeyCode::Numpad2) {
        picked = Some(1);
    } else if keys.just_pressed(KeyCode::Digit3) || keys.just_pressed(KeyCode::Numpad3) {
        picked = Some(2);
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

    let return_to = choices.return_state.unwrap_or(AppState::InGame);
    next_state.set(return_to);
}

pub fn cleanup_levelup_ui(mut commands: Commands, q: Query<Entity, With<LevelUpSelectUi>>) {
    for entity in &q {
        commands.entity(entity).despawn_recursive();
    }
}
