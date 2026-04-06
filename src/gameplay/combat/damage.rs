use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::coop::components::GhostState;
use crate::core::events::{DamageAppliedEvent, DamageEvent, DeathEvent};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::effects::{ArmorBroken, DashShieldBuff};
use crate::gameplay::combat::components::{Hurtbox, Knockback, Team};
use crate::gameplay::effects::flash::Flash;
use crate::gameplay::enemy::components::{
    BossCoreShield, BossDecoy, BossDirectionalDefense, TideHunterPhase, TideHunterState,
};
use crate::gameplay::player::components::{Health, InvincibilityTimer};
use crate::gameplay::skills::ChargeGainEvent;
use crate::ui::tutorial::{TutorialFlags, TutorialNotification};

pub fn apply_damage_events(
    mut commands: Commands,
    data: Option<Res<GameDataRegistry>>,
    mut damage_events: EventReader<DamageEvent>,
    mut applied_events: EventWriter<DamageAppliedEvent>,
    mut charge_events: EventWriter<ChargeGainEvent>,
    mut death_events: EventWriter<DeathEvent>,
    mut tutorial_flags: ResMut<TutorialFlags>,
    mut tutorial_ev: EventWriter<TutorialNotification>,
    directional_def_q: Query<&BossDirectionalDefense>,
    decoy_q: Query<(), With<BossDecoy>>,
    tide_hunter_q: Query<&TideHunterState>,
    core_shield_q: Query<&BossCoreShield>,
    mut q: Query<
        (
            Entity,
            &mut Health,
            Option<&mut InvincibilityTimer>,
            Option<&Hurtbox>,
            Option<&GhostState>,
            Option<&mut Flash>,
            Option<&ArmorBroken>,
            Option<&DashShieldBuff>,
            &mut Knockback,
            &GlobalTransform,
        ),
        Without<Replicated>,
    >,
) {
    for ev in damage_events.read() {
        if decoy_q.get(ev.target).is_ok() {
            continue;
        }

        let Ok((
            entity,
            mut health,
            inv_opt,
            hurtbox,
            ghost,
            flash_opt,
            armor_broken,
            dash_shield,
            mut knockback,
            tf,
        )) =
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

        if let Ok(shield) = core_shield_q.get(entity) {
            if shield.cores_alive > 0 {
                if !tutorial_flags.boss_cube_core_mechanic_shown {
                    tutorial_ev.send(TutorialNotification(
                        "护盾！先摧毁周围的子核心".to_string(),
                    ));
                    tutorial_flags.boss_cube_core_mechanic_shown = true;
                }
                continue;
            }
        }

        // DashShield: absorb one hit for player
        if dash_shield.is_some() && hurtbox.is_some_and(|h| h.team == Team::Player) {
            commands.entity(entity).remove::<DashShieldBuff>();
            continue;
        }

        let mut amount = ev.amount;

        if let Ok(defense) = directional_def_q.get(entity) {
            if ev.knockback.length_squared() > f32::EPSILON {
                let hit_from = -ev.knockback.normalize_or_zero();
                if hit_from.dot(defense.facing) > 0.4 {
                    amount *= 0.4;
                    if !tutorial_flags.boss_guardian_mechanic_shown {
                        tutorial_ev.send(TutorialNotification(
                            "正面有防御！绕到侧面或背后打弱点".to_string(),
                        ));
                        tutorial_flags.boss_guardian_mechanic_shown = true;
                    }
                }
            }
        }

        if let Ok(state) = tide_hunter_q.get(entity) {
            if state.phase == TideHunterPhase::Stunned {
                amount *= 1.5;
            }
        }
        if let Some(armor_broken) = armor_broken {
            amount *= armor_broken.damage_multiplier.max(1.0);
        }

        health.current = (health.current - amount).max(0.0);
        applied_events.send(DamageAppliedEvent {
            target: entity,
            source: ev.source,
            amount,
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
            death_events.send(DeathEvent {
                entity,
                source: ev.source,
                team,
            });
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
