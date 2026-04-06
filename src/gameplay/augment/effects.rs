use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::core::events::DamageAppliedEvent;
use crate::gameplay::combat::components::{DamageKind, Team};
use crate::gameplay::player::components::{DashState, Energy, Player};

use super::data::{AugmentId, AugmentInventory};

pub fn dash_energy_system(
    mut damage_events: EventReader<DamageAppliedEvent>,
    mut dash_hits: Local<HashMap<Entity, HashSet<Entity>>>,
    mut player_q: ParamSet<(
        Query<(Entity, &DashState), (With<Player>, Without<Replicated>)>,
        Query<
            (&DashState, Option<&AugmentInventory>, &mut Energy),
            (With<Player>, Without<Replicated>),
        >,
    )>,
) {
    let active_players: HashSet<Entity> = player_q
        .p0()
        .iter()
        .filter_map(|(player, dash)| dash.active.then_some(player))
        .collect();
    dash_hits.retain(|player, _| active_players.contains(player));

    // Collect events first to avoid borrow conflicts with ParamSet
    let relevant_events: Vec<_> = damage_events
        .read()
        .filter(|event| {
            event.kind == DamageKind::PlayerSkill
                && event.target_team == Some(Team::Enemy)
                && event.source.is_some()
        })
        .map(|event| (event.source.unwrap(), event.target))
        .collect();

    let mut p1 = player_q.p1();
    for (player, target) in relevant_events {
        let Ok((dash, inventory, mut energy)) = p1.get_mut(player) else {
            continue;
        };
        if !dash.active {
            continue;
        }

        let stacks = inventory
            .map(|value| value.stacks(AugmentId::DashEnergy))
            .unwrap_or(0);
        if stacks == 0 {
            continue;
        }

        let hit_set = dash_hits.entry(player).or_default();
        if !hit_set.insert(target) {
            continue;
        }

        let gain = if stacks >= 2 { 15.0 } else { 10.0 };
        energy.current = (energy.current + gain).min(energy.max);
    }
}
