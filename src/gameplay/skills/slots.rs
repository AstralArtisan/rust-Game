use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::gameplay::player::components::{Player, RewardModifiers, SkillSlot, SkillSlots};
use crate::gameplay::progression::floor::FloorNumber;

#[derive(Event, Debug, Clone, Copy)]
pub struct SkillUnlockedEvent {
    pub slot: SkillSlot,
}

pub fn sync_skill_unlocks(
    floor: Option<Res<FloorNumber>>,
    mut unlocked_events: EventWriter<SkillUnlockedEvent>,
    mut player_q: Query<(&RewardModifiers, &mut SkillSlots), (With<Player>, Without<Replicated>)>,
) {
    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
    for (mods, mut slots) in &mut player_q {
        if mods.ranged_mastery_stacks >= 2 && slots.unlock(SkillSlot::Two) {
            unlocked_events.send(SkillUnlockedEvent {
                slot: SkillSlot::Two,
            });
        }
        if floor_number >= 2 && slots.unlock(SkillSlot::Three) {
            unlocked_events.send(SkillUnlockedEvent {
                slot: SkillSlot::Three,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay::player::components::SkillType;

    #[test]
    fn skill_slots_default_to_first_slot_only() {
        let slots = SkillSlots::default();

        assert!(slots.state(SkillSlot::One).unlocked);
        assert_eq!(slots.state(SkillSlot::One).skill, Some(SkillType::SwordArc));
        assert!(!slots.state(SkillSlot::Two).unlocked);
        assert!(!slots.state(SkillSlot::Three).unlocked);
    }
}
