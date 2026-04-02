pub mod execute;
pub mod slots;

use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::core::events::DamageAppliedEvent;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::combat::components::{DamageKind, Team};
use crate::gameplay::player::components::{Energy, Player};
use crate::states::AppState;

pub use slots::SkillUnlockedEvent;

#[derive(Event, Debug, Clone, Copy)]
pub struct ChargeGainEvent {
    pub player: Entity,
    pub amount: f32,
}

pub struct SkillsPlugin;

impl Plugin for SkillsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ChargeGainEvent>()
            .add_event::<SkillUnlockedEvent>()
            .add_systems(
                Update,
                (
                    grant_charge_from_damage_system,
                    consume_charge_events,
                    slots::sync_skill_unlocks,
                    execute::activate_skill_inputs,
                    execute::advance_lock_on_mode,
                    execute::update_mark_indicators,
                    execute::update_homing_projectiles,
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

fn grant_charge_from_damage_system(
    data: Option<Res<GameDataRegistry>>,
    mut damage_events: EventReader<DamageAppliedEvent>,
    mut charge_events: EventWriter<ChargeGainEvent>,
) {
    let melee_gain = data
        .as_deref()
        .map(|value| value.player.melee_charge_gain)
        .unwrap_or(8.0);
    let ranged_gain = data
        .as_deref()
        .map(|value| value.player.ranged_charge_gain)
        .unwrap_or(4.0);

    for event in damage_events.read() {
        if event.attacker_team != Team::Player || event.target_team != Some(Team::Enemy) {
            continue;
        }
        let Some(player) = event.source else {
            continue;
        };

        let amount = match event.kind {
            DamageKind::PlayerMelee => melee_gain,
            DamageKind::PlayerRanged => ranged_gain,
            DamageKind::PlayerSkill | DamageKind::Enemy | DamageKind::Passive => 0.0,
        };
        if amount <= 0.0 {
            continue;
        }

        charge_events.send(ChargeGainEvent { player, amount });
    }
}

fn consume_charge_events(
    mut events: EventReader<ChargeGainEvent>,
    mut player_q: Query<&mut Energy, (With<Player>, Without<Replicated>)>,
) {
    for event in events.read() {
        let Ok(mut energy) = player_q.get_mut(event.player) else {
            continue;
        };
        energy.current = (energy.current + event.amount).clamp(0.0, energy.max);
    }
}
