use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::player::components::{Player, SkillSlot, SkillSlots, SkillType};
use crate::states::GamePhase;
use crate::ui::character_panel::{self, CharacterSummary, CharacterSummaryItem};
use crate::ui::feedback::{UiFeedbackEvent, UiFeedbackSeverity};
use crate::ui::widgets;

#[derive(Event, Debug, Clone, Copy)]
pub struct SkillEquippedEvent {
    pub skill: SkillType,
    pub slot: SkillSlot,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct SkillEquipCancelledEvent;

#[derive(Resource, Debug, Clone)]
pub struct SkillChoices {
    pub options: Vec<SkillChoiceOption>,
    pub return_state: Option<GamePhase>,
    pub step: SkillSelectStep,
}

impl Default for SkillChoices {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            return_state: None,
            step: SkillSelectStep::ChooseSkill,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SkillSelectStep {
    ChooseSkill,
    ReplaceSlot { option: SkillChoiceOption },
}

#[derive(Debug, Clone)]
pub struct SkillChoiceOption {
    pub skill: SkillType,
    pub title: String,
    pub description: String,
    pub energy_cost: f32,
    pub cooldown_s: f32,
}

#[derive(Component)]
pub struct SkillSelectUi;

#[derive(Component)]
pub struct SkillButton {
    pub index: usize,
}

#[derive(Component)]
pub struct SkillReplaceButton {
    pub slot: SkillSlot,
}

#[derive(Component)]
pub struct SkillCancelButton;

pub fn setup_skill_select_ui(
    mut commands: Commands,
    assets: Res<GameAssets>,
    choices: Res<SkillChoices>,
    player_q: Query<&SkillSlots, With<Player>>,
    data: Option<Res<GameDataRegistry>>,
    summary_q: Query<CharacterSummaryItem<'_>, With<Player>>,
) {
    let summary = character_panel::character_summary_from_query(&summary_q, data.as_deref());
    spawn_skill_select_root(
        &mut commands,
        &assets,
        &choices,
        player_q.get_single().ok().copied(),
        Some(&summary),
    );
}

fn spawn_skill_select_root(
    commands: &mut Commands,
    assets: &GameAssets,
    choices: &SkillChoices,
    slots: Option<SkillSlots>,
    summary: Option<&CharacterSummary>,
) {
    commands
        .spawn((
            widgets::overlay_root_node(),
            SkillSelectUi,
            Name::new("SkillSelectRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::adventure_panel_node(960.0))
                .with_children(|shell| {
                    shell.spawn(widgets::title_text(assets, "终结技配置", 26.0));
                    shell.spawn(widgets::muted_text(
                        assets,
                        "有空槽时优先装入空槽；满槽时由玩家选择替换或放弃。",
                        13.0,
                    ));
                    shell
                        .spawn(widgets::content_row_node())
                        .with_children(|row| {
                            if let Some(summary) = summary {
                                character_panel::spawn_character_summary(row, assets, summary);
                            }
                            row.spawn(widgets::card_node(600.0, 300.0, widgets::skill_color()))
                                .with_children(|panel| match &choices.step {
                                    SkillSelectStep::ChooseSkill => {
                                        panel.spawn(widgets::section_heading(assets, "获得终结技"));
                                        panel.spawn(widgets::muted_text(
                                            assets,
                                            "选择一项终结技；有空槽时会优先装入空槽",
                                            13.0,
                                        ));
                                        panel
                                            .spawn(NodeBundle {
                                                style: Style {
                                                    column_gap: Val::Px(12.0),
                                                    align_items: AlignItems::FlexStart,
                                                    margin: UiRect::top(Val::Px(8.0)),
                                                    ..default()
                                                },
                                                ..default()
                                            })
                                            .with_children(|row| {
                                                for (index, option) in
                                                    choices.options.iter().enumerate()
                                                {
                                                    spawn_skill_card(row, assets, index, option);
                                                }
                                            });
                                    }
                                    SkillSelectStep::ReplaceSlot { option } => {
                                        panel.spawn(widgets::section_heading(assets, "替换终结技"));
                                        panel.spawn(widgets::muted_text(
                                            assets,
                                            format!("{} 需要选择一个已解锁槽位替换", option.title),
                                            13.0,
                                        ));
                                        panel
                                            .spawn(NodeBundle {
                                                style: Style {
                                                    column_gap: Val::Px(10.0),
                                                    align_items: AlignItems::FlexStart,
                                                    margin: UiRect::top(Val::Px(8.0)),
                                                    ..default()
                                                },
                                                ..default()
                                            })
                                            .with_children(|row| {
                                                if let Some(slots) = slots {
                                                    for slot in SkillSlot::ALL {
                                                        let state = slots.state(slot);
                                                        if state.unlocked {
                                                            spawn_slot_card(
                                                                row,
                                                                assets,
                                                                slot,
                                                                state.skill,
                                                            );
                                                        }
                                                    }
                                                }
                                            });
                                        spawn_cancel_button(panel, assets);
                                    }
                                });
                        });
                });
        });
}

fn spawn_skill_card(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    index: usize,
    option: &SkillChoiceOption,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(185.0),
                    min_height: Val::Px(140.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.95)),
                border_color: BorderColor(Color::srgb(0.30, 0.72, 0.95)),
                ..default()
            },
            SkillButton { index },
        ))
        .with_children(|card| {
            card.spawn(TextBundle::from_section(
                format!("[{}]", index + 1),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 12.0,
                    color: Color::srgb(0.62, 0.68, 0.78),
                },
            ));
            card.spawn(TextBundle::from_section(
                format!("{} 能量 / {:.0}s CD", option.energy_cost, option.cooldown_s),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 11.0,
                    color: Color::srgb(0.30, 0.72, 0.95),
                },
            ));
            card.spawn(TextBundle::from_section(
                &option.title,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 17.0,
                    color: Color::WHITE,
                },
            ));
            card.spawn(TextBundle::from_section(
                &option.description,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 12.0,
                    color: Color::srgb(0.78, 0.82, 0.88),
                },
            ));
        });
}

fn spawn_slot_card(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    slot: SkillSlot,
    current: Option<SkillType>,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(150.0),
                    min_height: Val::Px(105.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.10, 0.12, 0.18, 0.95)),
                border_color: BorderColor(Color::srgb(0.78, 0.58, 0.28)),
                ..default()
            },
            SkillReplaceButton { slot },
        ))
        .with_children(|card| {
            card.spawn(widgets::title_text(
                assets,
                format!("槽位 {}", slot.key_label()),
                17.0,
            ));
            card.spawn(widgets::body_text(
                assets,
                current.map(SkillType::label).unwrap_or("空槽"),
                13.0,
            ));
        });
}

fn spawn_cancel_button(parent: &mut ChildBuilder, assets: &GameAssets) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(220.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(12.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.28, 0.18, 0.18)),
                ..default()
            },
            SkillCancelButton,
        ))
        .with_children(|button| {
            button.spawn(widgets::title_text(assets, "放弃", 16.0));
        });
}

pub fn skill_select_input(
    keys: Res<ButtonInput<KeyCode>>,
    assets: Res<GameAssets>,
    mut commands: Commands,
    mut choices: ResMut<SkillChoices>,
    mut player_q: Query<&mut SkillSlots, With<Player>>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut equipped_events: EventWriter<SkillEquippedEvent>,
    mut cancelled_events: EventWriter<SkillEquipCancelledEvent>,
    mut feedback: EventWriter<UiFeedbackEvent>,
    button_q: Query<
        (
            &Interaction,
            Option<&SkillButton>,
            Option<&SkillReplaceButton>,
            Option<&SkillCancelButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    ui_q: Query<Entity, With<SkillSelectUi>>,
) {
    match choices.step.clone() {
        SkillSelectStep::ChooseSkill => {
            let Some(index) = picked_skill_index(&keys, &button_q) else {
                return;
            };
            let Some(option) = choices.options.get(index).cloned() else {
                return;
            };
            let Ok(mut slots) = player_q.get_single_mut() else {
                return;
            };
            if let Some(slot) = slots.equip_empty_slot(option.skill) {
                equipped_events.send(SkillEquippedEvent {
                    skill: option.skill,
                    slot,
                });
                feedback.send(skill_equipped_feedback(
                    option.skill,
                    slot,
                    choices.return_state.unwrap_or(GamePhase::Playing),
                ));
                finish_skill_select(&mut choices, &mut next_state);
                return;
            }

            choices.step = SkillSelectStep::ReplaceSlot { option };
            let slots_snapshot = *slots;
            redraw_skill_select_ui(
                &mut commands,
                &assets,
                &choices,
                Some(slots_snapshot),
                None,
                &ui_q,
            );
        }
        SkillSelectStep::ReplaceSlot { option } => {
            if cancel_pressed(&keys, &button_q) {
                cancelled_events.send(SkillEquipCancelledEvent);
                feedback.send(UiFeedbackEvent {
                    title: "已放弃终结技".to_string(),
                    lines: vec!["原有终结技槽位保持不变。".to_string()],
                    severity: UiFeedbackSeverity::Info,
                    requires_ack: false,
                    return_phase: choices.return_state.unwrap_or(GamePhase::Playing),
                });
                finish_skill_select(&mut choices, &mut next_state);
                return;
            }

            let Some(slot) = picked_slot(&keys, &button_q) else {
                return;
            };
            let Ok(mut slots) = player_q.get_single_mut() else {
                return;
            };
            if slots.replace_slot(slot, option.skill) {
                equipped_events.send(SkillEquippedEvent {
                    skill: option.skill,
                    slot,
                });
                feedback.send(skill_equipped_feedback(
                    option.skill,
                    slot,
                    choices.return_state.unwrap_or(GamePhase::Playing),
                ));
                finish_skill_select(&mut choices, &mut next_state);
            }
        }
    }
}

fn picked_skill_index(
    keys: &ButtonInput<KeyCode>,
    button_q: &Query<
        (
            &Interaction,
            Option<&SkillButton>,
            Option<&SkillReplaceButton>,
            Option<&SkillCancelButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) -> Option<usize> {
    let mut picked = if keys.just_pressed(KeyCode::Digit1) || keys.just_pressed(KeyCode::Numpad1) {
        Some(0)
    } else if keys.just_pressed(KeyCode::Digit2) || keys.just_pressed(KeyCode::Numpad2) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit3) || keys.just_pressed(KeyCode::Numpad3) {
        Some(2)
    } else {
        None
    };

    for (interaction, button, _, _) in button_q.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(button) = button {
                picked = Some(button.index);
            }
        }
    }
    picked
}

fn picked_slot(
    keys: &ButtonInput<KeyCode>,
    button_q: &Query<
        (
            &Interaction,
            Option<&SkillButton>,
            Option<&SkillReplaceButton>,
            Option<&SkillCancelButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) -> Option<SkillSlot> {
    let mut picked = if keys.just_pressed(KeyCode::Digit1) || keys.just_pressed(KeyCode::Numpad1) {
        Some(SkillSlot::One)
    } else if keys.just_pressed(KeyCode::Digit2) || keys.just_pressed(KeyCode::Numpad2) {
        Some(SkillSlot::Two)
    } else if keys.just_pressed(KeyCode::Digit3) || keys.just_pressed(KeyCode::Numpad3) {
        Some(SkillSlot::Three)
    } else if keys.just_pressed(KeyCode::Digit4) || keys.just_pressed(KeyCode::Numpad4) {
        Some(SkillSlot::Four)
    } else {
        None
    };

    for (interaction, _, replace, _) in button_q.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(replace) = replace {
                picked = Some(replace.slot);
            }
        }
    }
    picked
}

fn cancel_pressed(
    keys: &ButtonInput<KeyCode>,
    button_q: &Query<
        (
            &Interaction,
            Option<&SkillButton>,
            Option<&SkillReplaceButton>,
            Option<&SkillCancelButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) -> bool {
    if keys.just_pressed(KeyCode::Escape) {
        return true;
    }
    button_q
        .iter()
        .any(|(interaction, _, _, cancel)| *interaction == Interaction::Pressed && cancel.is_some())
}

fn finish_skill_select(choices: &mut SkillChoices, next_state: &mut NextState<GamePhase>) {
    choices.step = SkillSelectStep::ChooseSkill;
    next_state.set(choices.return_state.unwrap_or(GamePhase::Playing));
}

fn redraw_skill_select_ui(
    commands: &mut Commands,
    assets: &GameAssets,
    choices: &SkillChoices,
    slots: Option<SkillSlots>,
    summary: Option<&CharacterSummary>,
    ui_q: &Query<Entity, With<SkillSelectUi>>,
) {
    for entity in ui_q.iter() {
        commands.entity(entity).despawn_recursive();
    }
    spawn_skill_select_root(commands, assets, choices, slots, summary);
}

pub fn cleanup_skill_select_ui(
    mut commands: Commands,
    q: Query<Entity, With<SkillSelectUi>>,
    mut choices: ResMut<SkillChoices>,
) {
    for entity in &q {
        commands.entity(entity).despawn_recursive();
    }
    choices.step = SkillSelectStep::ChooseSkill;
}

fn skill_equipped_feedback(
    skill: SkillType,
    slot: SkillSlot,
    return_phase: GamePhase,
) -> UiFeedbackEvent {
    UiFeedbackEvent::card(
        "终结技已配置",
        vec![format!(
            "{} 已装入 {} 槽。",
            skill.label(),
            slot.key_label()
        )],
        UiFeedbackSeverity::Success,
        return_phase,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_equipped_feedback_names_skill_and_slot() {
        let feedback =
            skill_equipped_feedback(SkillType::BladeDance, SkillSlot::Two, GamePhase::Playing);

        assert!(feedback.requires_ack);
        assert_eq!(feedback.title, "终结技已配置");
        assert!(feedback.lines.iter().any(|line| line.contains("剑舞")));
        assert!(feedback.lines.iter().any(|line| line.contains("2 槽")));
    }
}
