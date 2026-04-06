use bevy::prelude::*;

use crate::core::events::ScreenFlashRequest;
use crate::gameplay::map::InGameEntity;
use crate::utils::easing::ease_out_expo;

#[derive(Component)]
pub struct ScreenFlashOverlay {
    timer: Timer,
    base_alpha: f32,
}

pub fn spawn_screen_flash_overlay(mut commands: Commands) {
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            z_index: ZIndex::Global(999),
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
        ScreenFlashOverlay {
            timer: Timer::from_seconds(0.01, TimerMode::Once),
            base_alpha: 0.0,
        },
        InGameEntity,
        Name::new("ScreenFlashOverlay"),
    ));
}

pub fn screen_flash_receive_system(
    mut events: EventReader<ScreenFlashRequest>,
    mut q: Query<(&mut ScreenFlashOverlay, &mut BackgroundColor)>,
) {
    for req in events.read() {
        for (mut overlay, mut bg) in &mut q {
            overlay.timer = Timer::from_seconds(req.duration_s, TimerMode::Once);
            overlay.timer.reset();
            overlay.base_alpha = req.color.to_srgba().alpha.max(0.4);
            bg.0 = req.color.with_alpha(overlay.base_alpha);
        }
    }
}

pub fn clear_screen_flash(
    mut q: Query<(&mut ScreenFlashOverlay, &mut BackgroundColor)>,
) {
    for (mut overlay, mut bg) in &mut q {
        overlay.timer = Timer::from_seconds(0.01, TimerMode::Once);
        overlay.timer.set_elapsed(std::time::Duration::from_secs_f32(0.01));
        overlay.base_alpha = 0.0;
        bg.0 = Color::NONE;
    }
}

pub fn screen_flash_update_system(
    time: Res<Time<Real>>,
    mut q: Query<(&mut ScreenFlashOverlay, &mut BackgroundColor)>,
) {
    for (mut overlay, mut bg) in &mut q {
        overlay.timer.tick(time.delta());
        let t = overlay.timer.fraction();
        let alpha = overlay.base_alpha * (1.0 - ease_out_expo(t));
        bg.0 = bg.0.with_alpha(alpha);
    }
}
