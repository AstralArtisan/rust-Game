use bevy::prelude::*;

use crate::core::events::HitStopRequest;

#[derive(Resource, Default)]
pub struct HitStopState {
    remaining: f32,
}

pub fn hitstop_receive_system(
    mut events: EventReader<HitStopRequest>,
    mut state: ResMut<HitStopState>,
) {
    for req in events.read() {
        state.remaining = state.remaining.max(req.duration_s);
    }
}

pub fn hitstop_update_system(
    mut state: ResMut<HitStopState>,
    mut time: ResMut<Time<Virtual>>,
    real_time: Res<Time<Real>>,
) {
    if state.remaining > 0.0 {
        state.remaining -= real_time.delta_seconds();
        if state.remaining <= 0.0 {
            state.remaining = 0.0;
            time.set_relative_speed(1.0);
        } else {
            time.set_relative_speed(0.05);
        }
    }
}
