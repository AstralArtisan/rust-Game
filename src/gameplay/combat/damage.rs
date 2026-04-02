use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::coop::components::GhostState;
use crate::core::events::{DamageAppliedEvent, DamageEvent, DeathEvent};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::combat::components::{Hurtbox, Knockback, Team};
use crate::gameplay::effects::flash::Flash;
use crate::gameplay::player::components::{Health, InvincibilityTimer};
use crate::gameplay::skills::ChargeGainEvent;

pub fn apply_damage_events(
    data: Option<Res<GameDataRegistry>>,
    mut damage_events: EventReader<DamageEvent>,
    mut applied_events: EventWriter<DamageAppliedEvent>,
    mut charge_events: EventWriter<ChargeGainEvent>,
    mut death_events: EventWriter<DeathEvent>,
    mut q: Query<
        (
            Entity,
            &mut Health,
            Option<&mut InvincibilityTimer>,
            Option<&Hurtbox>,
            Option<&GhostState>,
            Option<&mut Flash>,
            &mut Knockback,
            &GlobalTransform,
        ),
        Without<Replicated>,
    >,
) {
    for ev in damage_events.read() {
        let Ok((entity, mut health, inv_opt, hurtbox, ghost, flash_opt, mut knockback, tf)) =
            q.get_mut(ev.target)
        else {
            continue;
        };

        if matches!(ghost, Some(GhostState::Ghost)) {
            continue;
        }

        if let Some(mut inv) = inv_opt {
            if !inv.timer.finished() {
                let perfect_dash_charge = data
                    .as_deref()
                    .map(|value| value.player.perfect_dash_charge_gain)
                    .unwrap_or(15.0);
                if hurtbox.is_some_and(|value| value.team == Team::Player) && ev.team == Team::Enemy
                {
                    charge_events.send(ChargeGainEvent {
                        player: entity,
                        amount: perfect_dash_charge,
                    });
                }
                continue;
            }
            inv.timer.reset();
        }

        health.current = (health.current - ev.amount).max(0.0);
        applied_events.send(DamageAppliedEvent {
            target: entity,
            source: ev.source,
            amount: ev.amount,
            attacker_team: ev.team,
            kind: ev.kind,
            target_team: hurtbox.map(|h| h.team),
            is_crit: ev.is_crit,
            pos: tf.translation().truncate(),
        });

        if let Some(mut flash) = flash_opt {
            flash.trigger(0.12);
        }

        knockback.0 = ev.knockback;

        if health.current <= 0.0 {
            let team = hurtbox.map(|h| h.team).unwrap_or(Team::Enemy);
            death_events.send(DeathEvent { entity, team });
        }
    }
}

pub fn apply_knockback_decay(
    time: Res<Time>,
    mut q: Query<(&mut Knockback, &mut Transform), Without<Replicated>>,
) {
    for (mut kb, mut tf) in &mut q {
        tf.translation += (kb.0 * time.delta_seconds()).extend(0.0);
        kb.0 *= 0.0;
    }
}
