use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::event_room::{ActiveEvent, EventPendingAction, EventUiAction};
use crate::gameplay::player::components::Player;
use crate::ui::character_panel::{self, CharacterSummaryItem};
use crate::ui::widgets;

#[derive(Component)]
pub struct EventRoomUi;

#[derive(Component)]
pub struct EventChoiceButton {
    pub index: usize,
}

pub fn setup_event_room_ui(
    mut commands: Commands,
    assets: Res<GameAssets>,
    event: Res<ActiveEvent>,
    data: Option<Res<GameDataRegistry>>,
    summary_q: Query<CharacterSummaryItem<'_>, With<Player>>,
) {
    let Some(event_type) = event.event_type else {
        return;
    };
    let summary = character_panel::character_summary_from_query(&summary_q, data.as_deref());

    commands
        .spawn((
            widgets::overlay_root_node(),
            EventRoomUi,
            Name::new("EventRoomUiRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::adventure_panel_node(960.0))
                .with_children(|panel| {
                    panel.spawn(widgets::title_text(
                        &assets,
                        format!("{} {}", event_type.symbol(), event_type.title()),
                        26.0,
                    ));
                    panel.spawn(widgets::muted_text(
                        &assets,
                        "点击选项或按数字键选择，Esc 放弃事件",
                        13.0,
                    ));
                    panel
                        .spawn(widgets::content_row_node())
                        .with_children(|row| {
                            character_panel::spawn_character_summary(row, &assets, &summary);
                            row.spawn(widgets::card_node(600.0, 340.0, event_type.accent_color()))
                                .with_children(|content| {
                                    content.spawn(widgets::body_text(
                                        &assets,
                                        event_type.description(),
                                        15.0,
                                    ));

                                    for (index, choice) in event.choices.iter().enumerate() {
                                        content
                                            .spawn((
                                                ButtonBundle {
                                                    style: Style {
                                                        width: Val::Percent(100.0),
                                                        min_height: Val::Px(62.0),
                                                        padding: UiRect::all(Val::Px(10.0)),
                                                        row_gap: Val::Px(5.0),
                                                        flex_direction: FlexDirection::Column,
                                                        align_items: AlignItems::FlexStart,
                                                        border: UiRect::all(Val::Px(1.0)),
                                                        ..default()
                                                    },
                                                    background_color: BackgroundColor(
                                                        widgets::section_color(),
                                                    ),
                                                    border_color: BorderColor(
                                                        event_type.accent_color(),
                                                    ),
                                                    ..default()
                                                },
                                                EventChoiceButton { index },
                                            ))
                                            .with_children(|button| {
                                                button.spawn(widgets::title_text(
                                                    &assets,
                                                    format!("{}. {}", index + 1, choice.label),
                                                    18.0,
                                                ));
                                                button.spawn(widgets::body_text(
                                                    &assets,
                                                    choice.description.clone(),
                                                    13.0,
                                                ));
                                            });
                                    }
                                });
                        });
                });
        });
}

pub fn event_room_ui_input_system(
    mut interaction_q: Query<
        (&Interaction, &EventChoiceButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut pending_action: ResMut<EventPendingAction>,
) {
    for (interaction, button, mut color) in &mut interaction_q {
        match *interaction {
            Interaction::Hovered => {
                color.0 = widgets::button_hover_color();
            }
            Interaction::None => {
                color.0 = widgets::section_color();
            }
            Interaction::Pressed => {
                pending_action.0 = Some(EventUiAction::Select(button.index));
            }
        }
    }
}

pub fn cleanup_event_room_ui(mut commands: Commands, q: Query<Entity, With<EventRoomUi>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}
