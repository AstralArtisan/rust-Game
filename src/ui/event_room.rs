use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::gameplay::event_room::ActiveEvent;
use crate::ui::widgets;

#[derive(Component)]
pub struct EventRoomUi;

pub fn setup_event_room_ui(
    mut commands: Commands,
    assets: Res<GameAssets>,
    event: Res<ActiveEvent>,
) {
    let Some(event_type) = event.event_type else {
        return;
    };

    commands
        .spawn((
            widgets::scrim_node(0.40),
            EventRoomUi,
            Name::new("EventRoomUiRoot"),
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(700.0),
                    max_width: Val::Percent(92.0),
                    align_items: AlignItems::Stretch,
                    flex_direction: FlexDirection::Row,
                    ..default()
                },
                background_color: BackgroundColor(widgets::panel_color()),
                ..default()
            })
            .with_children(|panel| {
                panel.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(8.0),
                        align_self: AlignSelf::Stretch,
                        ..default()
                    },
                    background_color: BackgroundColor(event_type.accent_color()),
                    ..default()
                });

                panel
                    .spawn(NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            padding: UiRect::all(Val::Px(18.0)),
                            row_gap: Val::Px(12.0),
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|content| {
                        content.spawn(widgets::title_text(
                            &assets,
                            format!("{} {}", event_type.symbol(), event_type.title()),
                            30.0,
                        ));
                        content.spawn(widgets::body_text(&assets, event_type.description(), 18.0));
                        content.spawn(widgets::muted_text(
                            &assets,
                            "按数字键选择选项，Esc 放弃事件",
                            15.0,
                        ));

                        for (index, choice) in event.choices.iter().enumerate() {
                            content
                                .spawn(widgets::section_node(widgets::section_color()))
                                .with_children(|section| {
                                    section.spawn(widgets::title_text(
                                        &assets,
                                        format!("{}. {}", index + 1, choice.label),
                                        22.0,
                                    ));
                                    section.spawn(widgets::body_text(
                                        &assets,
                                        choice.description.clone(),
                                        16.0,
                                    ));
                                });
                        }
                    });
            });
        });
}

pub fn cleanup_event_room_ui(mut commands: Commands, q: Query<Entity, With<EventRoomUi>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}
