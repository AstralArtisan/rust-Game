use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::gameplay::player::components::{Player, SkillSlot, SkillSlots};
use crate::gameplay::progression::floor::FloorNumber;

#[derive(Event, Debug, Clone, Copy)]
pub struct SkillUnlockedEvent {
    pub slot: SkillSlot,
}

pub fn sync_skill_unlocks(
    floor: Option<Res<FloorNumber>>,
    mut unlocked_events: EventWriter<SkillUnlockedEvent>,
    mut player_q: Query<&mut SkillSlots, (With<Player>, Without<Replicated>)>,
) {
    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
    for mut slots in &mut player_q {
        if floor_number >= 2 && slots.unlock(SkillSlot::Two) {
            unlocked_events.send(SkillUnlockedEvent {
                slot: SkillSlot::Two,
            });
        }
        if floor_number >= 3 && slots.unlock(SkillSlot::Three) {
            unlocked_events.send(SkillUnlockedEvent {
                slot: SkillSlot::Three,
            });
        }
        if floor_number >= 4 && slots.unlock(SkillSlot::Four) {
            unlocked_events.send(SkillUnlockedEvent {
                slot: SkillSlot::Four,
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
        assert_eq!(
            slots.state(SkillSlot::One).skill,
            Some(SkillType::GroundSlam)
        );
        assert!(!slots.state(SkillSlot::Two).unlocked);
        assert!(!slots.state(SkillSlot::Three).unlocked);
        assert!(!slots.state(SkillSlot::Four).unlocked);
    }

    #[test]
    fn equip_empty_slot_unlocks_empty_slot_without_replacing_existing_skill() {
        let mut slots = SkillSlots::default();

        let equipped = slots.equip_empty_slot(SkillType::FrostField);

        assert_eq!(equipped, Some(SkillSlot::Two));
        assert_eq!(
            slots.state(SkillSlot::One).skill,
            Some(SkillType::GroundSlam)
        );
        assert_eq!(
            slots.state(SkillSlot::Two).skill,
            Some(SkillType::FrostField)
        );
        assert!(slots.state(SkillSlot::Two).unlocked);
    }

    #[test]
    fn full_slots_require_explicit_replacement() {
        let mut slots = SkillSlots::default();
        assert_eq!(
            slots.equip_empty_slot(SkillType::WarCry),
            Some(SkillSlot::Two)
        );
        assert_eq!(
            slots.equip_empty_slot(SkillType::BulletBarrage),
            Some(SkillSlot::Three)
        );
        assert_eq!(
            slots.equip_empty_slot(SkillType::MeteorFall),
            Some(SkillSlot::Four)
        );

        let equipped = slots.equip_empty_slot(SkillType::FrostField);

        assert_eq!(equipped, None);
        assert_eq!(
            slots.state(SkillSlot::One).skill,
            Some(SkillType::GroundSlam)
        );
        assert_eq!(slots.state(SkillSlot::Two).skill, Some(SkillType::WarCry));

        assert!(slots.replace_slot(SkillSlot::Two, SkillType::FrostField));
        assert_eq!(
            slots.state(SkillSlot::Two).skill,
            Some(SkillType::FrostField)
        );
    }
}
