use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentGrantResult, AugmentRarity};
use crate::states::GamePhase;
use crate::ui::widgets;
use crate::utils::entity::safe_despawn_recursive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiFeedbackSeverity {
    Info,
    Success,
    Warning,
}

#[derive(Event, Debug, Clone)]
pub struct UiFeedbackEvent {
    pub title: String,
    pub lines: Vec<String>,
    pub severity: UiFeedbackSeverity,
    pub requires_ack: bool,
    pub return_phase: GamePhase,
}

impl UiFeedbackEvent {
    pub fn toast(title: impl Into<String>, lines: impl Into<Vec<String>>) -> Self {
        Self {
            title: title.into(),
            lines: lines.into(),
            severity: UiFeedbackSeverity::Info,
            requires_ack: false,
            return_phase: GamePhase::Playing,
        }
    }

    pub fn card(
        title: impl Into<String>,
        lines: impl Into<Vec<String>>,
        severity: UiFeedbackSeverity,
        return_phase: GamePhase,
    ) -> Self {
        Self {
            title: title.into(),
            lines: lines.into(),
            severity,
            requires_ack: true,
            return_phase,
        }
    }
}

#[derive(Resource, Debug, Default, Clone)]
pub struct ActiveUiFeedback {
    pub current: Option<UiFeedbackEvent>,
}

#[derive(Component)]
pub struct FeedbackRoot;

#[derive(Component)]
pub struct FeedbackCardUi;

#[derive(Component)]
pub struct FeedbackAckButton;

#[derive(Component, Debug, Clone)]
pub struct FeedbackToast {
    pub timer: Timer,
}

pub fn ensure_feedback_root(mut commands: Commands, existing: Query<(), With<FeedbackRoot>>) {
    if existing.iter().next().is_some() {
        return;
    }
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(56.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            ..default()
        },
        FeedbackRoot,
        Name::new("FeedbackRoot"),
    ));
}

pub fn handle_ui_feedback_events(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    root_q: Query<Entity, With<FeedbackRoot>>,
    mut feedback: EventReader<UiFeedbackEvent>,
    mut active: ResMut<ActiveUiFeedback>,
    mut next_phase: ResMut<NextState<GamePhase>>,
) {
    let Some(assets) = assets else { return };
    let Ok(root) = root_q.get_single() else {
        return;
    };

    for event in feedback.read() {
        if event.requires_ack {
            active.current = Some(event.clone());
            next_phase.set(GamePhase::Feedback);
        } else {
            spawn_feedback_toast(&mut commands, root, &assets, event);
        }
    }
}

fn spawn_feedback_toast(
    commands: &mut Commands,
    root: Entity,
    assets: &GameAssets,
    event: &UiFeedbackEvent,
) {
    let accent = severity_color(event.severity);
    commands.entity(root).with_children(|root| {
        root.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(520.0),
                    max_width: Val::Percent(90.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(widgets::panel_color()),
                border_color: BorderColor(accent),
                ..default()
            },
            FeedbackToast {
                timer: Timer::from_seconds(2.4, TimerMode::Once),
            },
            Name::new("FeedbackToast"),
        ))
        .with_children(|toast| {
            toast.spawn(widgets::accent_text(assets, &event.title, 16.0, accent));
            for line in event.lines.iter().take(3) {
                toast.spawn(widgets::body_text(assets, line, 13.0));
            }
        });
    });
}

pub fn setup_feedback_card(
    mut commands: Commands,
    assets: Res<GameAssets>,
    active: Res<ActiveUiFeedback>,
) {
    let Some(event) = active.current.as_ref() else {
        return;
    };
    let accent = severity_color(event.severity);
    commands
        .spawn((
            widgets::overlay_root_node(),
            FeedbackCardUi,
            Name::new("FeedbackCardRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::adventure_panel_node(620.0))
                .with_children(|panel| {
                    panel.spawn(widgets::accent_text(&assets, &event.title, 30.0, accent));
                    for line in &event.lines {
                        panel.spawn(widgets::body_text(&assets, line, 17.0));
                    }
                    panel
                        .spawn((widgets::button_bundle_sized(220.0, 48.0), FeedbackAckButton))
                        .with_children(|button| {
                            button.spawn(widgets::title_text(&assets, "确认", 19.0));
                        });
                    panel.spawn(widgets::muted_text(
                        &assets,
                        "Enter / Space / Esc 继续",
                        13.0,
                    ));
                });
        });
}

pub fn feedback_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut interaction_q: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<FeedbackAckButton>),
    >,
    mut active: ResMut<ActiveUiFeedback>,
    mut next_phase: ResMut<NextState<GamePhase>>,
) {
    let mut confirmed = keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::Escape);

    for (interaction, mut color) in &mut interaction_q {
        widgets::apply_button_interaction(*interaction, &mut color, widgets::button_base_color());
        if *interaction == Interaction::Pressed {
            confirmed = true;
        }
    }

    if !confirmed {
        return;
    }

    let return_phase = active
        .current
        .as_ref()
        .map(|event| event.return_phase)
        .unwrap_or(GamePhase::Playing);
    active.current = None;
    next_phase.set(return_phase);
}

pub fn update_feedback_toasts(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut FeedbackToast, &mut BackgroundColor)>,
) {
    for (entity, mut toast, mut bg) in &mut q {
        toast.timer.tick(time.delta());
        let alpha = (1.0 - toast.timer.fraction()).clamp(0.0, 1.0);
        bg.0.set_alpha(alpha * 0.94);
        if toast.timer.finished() {
            safe_despawn_recursive(&mut commands, entity);
        }
    }
}

pub fn cleanup_feedback_card(mut commands: Commands, q: Query<Entity, With<FeedbackCardUi>>) {
    for entity in &q {
        safe_despawn_recursive(&mut commands, entity);
    }
}

pub fn severity_color(severity: UiFeedbackSeverity) -> Color {
    match severity {
        UiFeedbackSeverity::Info => Color::srgb(0.38, 0.66, 1.0),
        UiFeedbackSeverity::Success => Color::srgb(0.38, 0.82, 0.46),
        UiFeedbackSeverity::Warning => widgets::gold_color(),
    }
}

pub fn augment_grant_lines(
    grant: AugmentGrantResult,
    data: Option<&GameDataRegistry>,
) -> Vec<String> {
    let (title, rarity, effect) = data
        .and_then(|registry| {
            registry
                .augments
                .augments
                .iter()
                .find(|augment| augment.id == grant.id)
                .map(|augment| {
                    (
                        augment.title.as_str(),
                        augment.rarity,
                        augment.description_for_stacks(grant.after_stacks),
                    )
                })
        })
        .unwrap_or(("未知强化", AugmentRarity::Common, "效果未配置"));

    let change = if grant.before_stacks == 0 {
        format!(
            "获得强化：{} · {} Lv{}",
            title,
            widgets::rarity_label(rarity),
            grant.after_stacks
        )
    } else if grant.after_stacks > grant.before_stacks {
        format!(
            "强化升级：{} Lv{} -> Lv{}",
            title, grant.before_stacks, grant.after_stacks
        )
    } else {
        format!("强化已达上限：{} Lv{}", title, grant.after_stacks)
    };

    vec![change, format!("效果：{effect}")]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay::augment::data::AugmentId;

    #[test]
    fn card_feedback_requires_ack_and_returns_to_phase() {
        let event = UiFeedbackEvent::card(
            "获得强化",
            vec!["连锁闪电 Lv1".to_string()],
            UiFeedbackSeverity::Success,
            GamePhase::Playing,
        );

        assert!(event.requires_ack);
        assert_eq!(event.return_phase, GamePhase::Playing);
    }

    #[test]
    fn augment_feedback_reports_level_change() {
        let lines = augment_grant_lines(
            AugmentGrantResult {
                id: AugmentId::Piercing,
                before_stacks: 1,
                after_stacks: 2,
                reached_cap: false,
            },
            None,
        );

        assert!(lines.iter().any(|line| line.contains("Lv1 -> Lv2")));
    }
}
