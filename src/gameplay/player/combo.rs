use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::core::events::DamageAppliedEvent;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::combat::components::Team;
use crate::gameplay::player::components::{Combo, Player};
use crate::gameplay::skills::ChargeGainEvent;

pub fn update_combo_state(
    time: Res<Time>,
    data: Option<Res<GameDataRegistry>>,
    mut damage_applied: EventReader<DamageAppliedEvent>,
    mut charge_events: EventWriter<ChargeGainEvent>,
    mut player_q: Query<(Entity, &mut Combo), (With<Player>, Without<Replicated>)>,
) {
    let events = damage_applied.read().cloned().collect::<Vec<_>>();
    let combo_charge_gain = data
        .as_deref()
        .map(|value| value.player.combo_charge_gain)
        .unwrap_or(10.0);
    for (player_e, mut combo) in &mut player_q {
        combo.timer.tick(time.delta());
        if combo.timer.finished() {
            combo.count = 0;
        }

        for ev in &events {
            if ev.attacker_team == Team::Player {
                let previous = combo.count;
                combo.count = combo.count.saturating_add(1);
                combo.timer.reset();
                if previous / 10 != combo.count / 10 && combo.count >= 10 {
                    charge_events.send(ChargeGainEvent {
                        player: player_e,
                        amount: combo_charge_gain,
                    });
                }
            }
            if ev.attacker_team == Team::Enemy && ev.target == player_e {
                combo.count = 0;
                combo.timer.reset();
            }
        }
    }
}
