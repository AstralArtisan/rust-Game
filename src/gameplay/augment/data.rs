#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AugmentRarity {
    Common,
    Elite,
    Legendary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AugmentCategory {
    Melee,
    Ranged,
    Mobility,
    General,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AugmentId {
    // Melee (8)
    LifestealSlash,
    HeavyStrike,
    ComboAccelerate,
    Whirlwind,
    ArmorBreak,
    Reflect,
    SwordWave,
    Executioner,
    // Ranged (8)
    Piercing,
    SpeedBoost,
    ExtraProjectile,
    Homing,
    ChainLightning,
    Scatter,
    BulletStorm,
    Freeze,
    // Mobility (6)
    DashTrail,
    DashEnergy,
    ExtendedInvuln,
    DashReset,
    DashShield,
    Blink,
    // General (8)
    GoldBonus,
    XpBonus,
    PickupRange,
    Thorns,
    KillHeal,
    CritEnhance,
    Phoenix,
    Greed,
}

/// A single held augment with its stack count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeldAugment {
    pub id: AugmentId,
    pub stacks: u8, // 1 = base, 2 = upgraded, 3 = qualitative capstone
}

/// Player component: tracks all collected augments this run.
/// Replaces the old RuneLoadout.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct AugmentInventory {
    pub augments: Vec<HeldAugment>,
}

impl AugmentInventory {
    pub const MAX_STACKS: u8 = 3;

    /// Add an augment. If already held, increment stacks (max 3).
    pub fn add(&mut self, id: AugmentId) {
        if let Some(held) = self.augments.iter_mut().find(|a| a.id == id) {
            held.stacks = (held.stacks + 1).min(Self::MAX_STACKS);
        } else {
            self.augments.push(HeldAugment { id, stacks: 1 });
        }
    }

    pub fn has(&self, id: AugmentId) -> bool {
        self.augments.iter().any(|a| a.id == id)
    }

    pub fn stacks(&self, id: AugmentId) -> u8 {
        self.augments
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.stacks)
            .unwrap_or(0)
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.augments.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_augment() {
        let mut inv = AugmentInventory::default();
        inv.add(AugmentId::Piercing);
        assert!(inv.has(AugmentId::Piercing));
        assert_eq!(inv.stacks(AugmentId::Piercing), 1);
    }

    #[test]
    fn test_upgrade_augment() {
        let mut inv = AugmentInventory::default();
        inv.add(AugmentId::Piercing);
        inv.add(AugmentId::Piercing);
        assert_eq!(inv.stacks(AugmentId::Piercing), 2);
    }

    #[test]
    fn test_max_stacks() {
        let mut inv = AugmentInventory::default();
        inv.add(AugmentId::Piercing);
        inv.add(AugmentId::Piercing);
        inv.add(AugmentId::Piercing);
        inv.add(AugmentId::Piercing);
        assert_eq!(inv.stacks(AugmentId::Piercing), 3);
    }

    #[test]
    fn test_count() {
        let mut inv = AugmentInventory::default();
        inv.add(AugmentId::Piercing);
        inv.add(AugmentId::GoldBonus);
        inv.add(AugmentId::DashTrail);
        assert_eq!(inv.count(), 3);
    }

    #[test]
    fn test_has_returns_false() {
        let inv = AugmentInventory::default();
        assert!(!inv.has(AugmentId::Phoenix));
    }
}
