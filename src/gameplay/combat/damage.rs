use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::coop::components::GhostState;
use crate::core::events::{DamageAppliedEvent, DamageEvent, DeathEvent};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::effects::{ArmorBroken, DashShieldBuff, Frozen};
use crate::gameplay::combat::components::{Hurtbox, Knockback, Team};
use crate::gameplay::effects::flash::Flash;
use crate::gameplay::enemy::components::{
    BossCoreShield, BossDirectionalDefense, ShieldedAffixState, TideHunterPhase, TideHunterState,
};
use crate::gameplay::player::components::{Health, InvincibilityTimer, RewardModifiers};
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
            Option<&mut DashShieldBuff>,
            Option<&mut Frozen>,
            Option<&mut ShieldedAffixState>,
            Option<&mut RewardModifiers>,
            &mut Knockback,
            &GlobalTransform,
        ),
        Without<Replicated>,
    >,
) {
    for ev in damage_events.read() {
        let Ok((
            entity,
            mut health,
            inv_opt,
            hurtbox,
            ghost,
            flash_opt,
            armor_broken,
            dash_shield,
            frozen,
            shielded_affix,
            talisman_mods,
            mut knockback,
            tf,
        )) = q.get_mut(ev.target)
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
                    tutorial_ev.send(TutorialNotification("护盾！先摧毁周围的子核心".to_string()));
                    tutorial_flags.boss_cube_core_mechanic_shown = true;
                }
                continue;
            }
        }

        // DashShield: absorb one hit for player
        if let Some(mut dash_shield) = dash_shield {
            if hurtbox.is_some_and(|h| h.team == Team::Player) {
                dash_shield.charges = dash_shield.charges.saturating_sub(1);
                if dash_shield.charges == 0 {
                    let _break_damage_fraction = dash_shield.break_damage_fraction;
                    commands.entity(entity).remove::<DashShieldBuff>();
                }
                continue;
            }
        }

        let mut amount = ev.amount;

        if let Some(frozen) = frozen {
            if ev.team == Team::Player && frozen.shatter_damage_bonus > 0.0 {
                amount *= 1.0 + frozen.shatter_damage_bonus;
                commands.entity(entity).remove::<Frozen>();
            }
        }

        if let Some(mut shielded) = shielded_affix {
            if shielded.charges > 0 {
                shielded.charges -= 1;
                if shielded.charges == 0 {
                    commands.entity(entity).remove::<ShieldedAffixState>();
                }
                continue;
            }
        }

        if let Ok(defense) = directional_def_q.get(entity) {
            if ev.knockback.length_squared() > f32::EPSILON {
                let hit_from = -ev.knockback.normalize_or_zero();
                if hit_from.dot(defense.facing) > 0.4 {
                    amount *= 0.4;
                    if !tutorial_flags.boss_guardian_mechanic_shown {
                        tutorial_ev.send(TutorialNotification(
                            "Boss 正面防御极高，绕到背后输出！".to_string(),
                        ));
                        tutorial_flags.boss_guardian_mechanic_shown = true;
                    }
                } else if hit_from.dot(defense.facing) < -0.35 {
                    amount *= 1.35;
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
            if ev.is_crit && armor_broken.crit_taken_bonus > 0.0 {
                amount *= 1.0 + armor_broken.crit_taken_bonus;
            }
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

        knockback.0 = ev.knockback;
        if let Some(mut flash) = flash_opt {
            flash.trigger(0.12);
        }

        let prevented_death = talisman_mods
            .map(|mut mods| apply_talisman_lethal_guard(&mut health, &mut mods))
            .unwrap_or(false);
        if health.current <= 0.0 && !prevented_death {
            death_events.send(DeathEvent {
                entity,
                source: ev.source,
                team: hurtbox.map(|h| h.team).unwrap_or(Team::Enemy),
            });
        }
    }
}

fn apply_talisman_lethal_guard(health: &mut Health, mods: &mut RewardModifiers) -> bool {
    if health.current > 0.0 || mods.talisman_charges == 0 {
        return false;
    }
    mods.talisman_charges -= 1;
    health.current = 1.0;
    true
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn talisman_consumes_charge_and_prevents_lethal_damage() {
        let mut health = Health {
            current: 0.0,
            max: 100.0,
        };
        let mut mods = RewardModifiers {
            talisman_charges: 1,
            ..Default::default()
        };

        assert!(apply_talisman_lethal_guard(&mut health, &mut mods));
        assert_eq!(health.current, 1.0);
        assert_eq!(mods.talisman_charges, 0);
    }

    #[test]
    fn talisman_does_not_fire_without_charge() {
        let mut health = Health {
            current: 0.0,
            max: 100.0,
        };
        let mut mods = RewardModifiers::default();

        assert!(!apply_talisman_lethal_guard(&mut health, &mut mods));
        assert_eq!(health.current, 0.0);
    }
}
