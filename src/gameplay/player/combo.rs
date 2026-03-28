use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::core::events::DamageAppliedEvent;
use crate::gameplay::combat::components::Team;
use crate::gameplay::player::components::{Combo, Player};

pub fn update_combo_state(
    time: Res<Time>,
    mut damage_applied: EventReader<DamageAppliedEvent>,
    mut player_q: Query<(Entity, &mut Combo), (With<Player>, Without<Replicated>)>,
) {
    let events = damage_applied.read().cloned().collect::<Vec<_>>();
    for (player_e, mut combo) in &mut player_q {
        combo.timer.tick(time.delta());
        if combo.timer.finished() {
            combo.count = 0;
        }

        for ev in &events {
            if ev.attacker_team == Team::Player {
                combo.count = combo.count.saturating_add(1);
                combo.timer.reset();
            }
            if ev.attacker_team == Team::Enemy && ev.target == player_e {
                combo.count = 0;
                combo.timer.reset();
            }
        }
    }
}
