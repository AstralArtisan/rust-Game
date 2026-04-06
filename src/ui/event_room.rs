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
            widgets::scrim_node(0.62),
            EventRoomUi,
            Name::new("EventRoomUiRoot"),
        ))
        .with_children(|root| {
            root.spawn(widgets::modal_panel_node(700.0))
                .with_children(|panel| {
                    panel.spawn(widgets::title_text(&assets, event_type.title(), 30.0));
                    panel.spawn(widgets::body_text(&assets, event_type.description(), 18.0));
                    panel.spawn(widgets::muted_text(
                        &assets,
                        "按数字键选择选项，Esc 放弃事件",
                        15.0,
                    ));

                    for (index, choice) in event.choices.iter().enumerate() {
                        panel
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
}

pub fn cleanup_event_room_ui(mut commands: Commands, q: Query<Entity, With<EventRoomUi>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}
