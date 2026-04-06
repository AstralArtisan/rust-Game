use bevy::prelude::*;

#[allow(dead_code)]
#[allow(dead_code)]
pub fn tick_timer(timer: &mut Timer, time: &Time) -> bool {
    timer.tick(time.delta());
    timer.just_finished()
}
