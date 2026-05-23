use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::data::definitions::RewardScalingConfig;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::player::components::*;
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::rewards::apply::heal_amount;
use crate::states::GamePhase;
use crate::ui::character_panel::{self, CharacterSummaryItem};
use crate::ui::feedback::{UiFeedbackEvent, UiFeedbackSeverity};
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
    summary_q: Query<CharacterSummaryItem<'_>, With<Player>>,
    floor: Option<Res<FloorNumber>>,
    data: Option<Res<GameDataRegistry>>,
) {
    let summary = character_panel::character_summary_from_query(&summary_q, data.as_deref());
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
            widgets::overlay_root_node(),
            LevelUpSelectUi,
            Name::new("LevelUpRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::adventure_panel_node(980.0))
                .with_children(|panel| {
                    panel.spawn(widgets::title_text(
                        &assets,
                        format!("升级！ Lv.{}", choices.new_level),
                        26.0,
                    ));
                    panel.spawn(widgets::title_text(
                        &assets,
                        "选择回血恢复状态，或选择一项属性提升",
                        14.0,
                    ));
                    panel
                        .spawn(widgets::content_row_node())
                        .with_children(|row| {
                            character_panel::spawn_character_summary(row, &assets, &summary);
                            row.spawn(widgets::card_node(620.0, 320.0, widgets::energy_color()))
                                .with_children(|choices_panel| {
                                    choices_panel
                                        .spawn(widgets::panel_node(widgets::section_color()))
                                        .with_children(|col| {
                                            col.spawn(widgets::title_text(&assets, "回血", 21.0));
                                            spawn_levelup_heal_card(col, &assets, heal_value);
                                        });

                                    choices_panel
                                        .spawn(widgets::panel_node(widgets::section_alt_color()))
                                        .with_children(|col| {
                                            col.spawn(widgets::title_text(
                                                &assets,
                                                "属性强化",
                                                21.0,
                                            ));
                                            for (i, opt) in
                                                choices.options.iter().enumerate().skip(1).take(3)
                                            {
                                                spawn_levelup_card(col, &assets, i, opt);
                                            }
                                        });
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
                    height: Val::Px(170.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(6.0),
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
            button.spawn(widgets::title_text(assets, "1. 回血", 21.0));
            button.spawn(widgets::title_text(
                assets,
                format!("恢复 {:.0} 生命", heal_value),
                19.0,
            ));
            button.spawn(widgets::body_text(assets, "稳住当前状态后继续推进", 13.0));
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
                    width: Val::Px(280.0),
                    min_height: Val::Px(68.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
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
                    font_size: 12.0,
                    color: Color::srgb(0.6, 0.7, 0.6),
                },
            ));
            card.spawn(TextBundle::from_section(
                &opt.label,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 17.0,
                    color: Color::srgb(0.40, 0.90, 0.40),
                },
            ));
            card.spawn(TextBundle::from_section(
                &opt.description,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 12.0,
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
            &mut RangedCooldown,
            &mut DashCooldown,
            &RewardModifiers,
        ),
        With<Player>,
    >,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut feedback: EventWriter<UiFeedbackEvent>,
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

    let mut feedback_line = opt.label.clone();
    if let Ok((
        mut health,
        mut atk,
        mut spd,
        mut crit,
        mut atk_cd,
        mut ranged_cd,
        mut dash_cd,
        mods,
    )) = player_q.get_single_mut()
    {
        match opt.apply {
            LevelUpStat::AttackPower(v) => {
                atk.0 += v;
                feedback_line = format!("攻击力 +{v:.0}，当前 {:.0}", atk.0);
            }
            LevelUpStat::MaxHealth(v) => {
                health.max += v;
                health.current += v;
                feedback_line = format!("生命上限 +{v:.0}，当前 {:.0}", health.max);
            }
            LevelUpStat::RecoverHealth(amount) => {
                let before = health.current;
                health.current = (health.current + amount).min(health.max);
                feedback_line = format!("生命: {:.0} -> {:.0}", before, health.current);
            }
            LevelUpStat::MoveSpeed(v) => {
                spd.0 += v;
                feedback_line = format!("移动速度 +{v:.0}，当前 {:.0}", spd.0);
            }
            LevelUpStat::CritChance(v) => {
                crit.0 = (crit.0 + v).min(0.80);
                feedback_line = format!("暴击率 +{:.0}%，当前 {:.0}%", v * 100.0, crit.0 * 100.0);
            }
            LevelUpStat::AttackSpeed(v) => {
                // Permanently shrink the base cooldown — anything that re-applies
                // a buff (player_attack_input_system, save load, coop sync) reads
                // base_duration_s, so changing only timer.duration would be
                // silently reverted on the next attack. design.md groups melee
                // and ranged as a single "attack speed" axis, so trim both.
                atk_cd.base_duration_s = (atk_cd.base_duration_s - v).max(0.15);
                ranged_cd.base_duration_s = (ranged_cd.base_duration_s - v).max(0.15);
                atk_cd.apply_speed_bonus(mods.total_melee_speed_bonus());
                ranged_cd.apply_speed_bonus(mods.total_ranged_speed_bonus());
                feedback_line = format!(
                    "攻击间隔 -{v:.2}s，近战 {:.2}s / 远程 {:.2}s",
                    atk_cd.base_duration_s, ranged_cd.base_duration_s
                );
            }
            LevelUpStat::DashCooldown(v) => {
                // Same as above: shrink base_duration_s so the change survives the
                // next apply_reduction call from save load or augment effects.
                dash_cd.base_duration_s = (dash_cd.base_duration_s - v).max(0.3);
                dash_cd.apply_reduction(mods.total_dash_cooldown_reduction());
                feedback_line = format!("冲刺冷却 -{v:.2}s，当前 {:.2}s", dash_cd.base_duration_s);
            }
        }
    }

    let return_to = choices.return_state.unwrap_or(GamePhase::Playing);
    feedback.send(UiFeedbackEvent::card(
        "升级结算",
        vec![feedback_line],
        UiFeedbackSeverity::Success,
        return_to,
    ));
    next_state.set(return_to);
}

pub fn cleanup_levelup_ui(mut commands: Commands, q: Query<Entity, With<LevelUpSelectUi>>) {
    for entity in &q {
        commands.entity(entity).despawn_recursive();
    }
}
