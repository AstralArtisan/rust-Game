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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AugmentGrantResult {
    pub id: AugmentId,
    pub before_stacks: u8,
    pub after_stacks: u8,
    pub reached_cap: bool,
}

impl AugmentInventory {
    pub const MAX_STACKS: u8 = 3;

    /// Add an augment. If already held, increment stacks (max 3).
    pub fn add(&mut self, id: AugmentId) {
        let _ = self.grant(id);
    }

    pub fn grant(&mut self, id: AugmentId) -> AugmentGrantResult {
        let before_stacks = self.stacks(id);
        if let Some(held) = self.augments.iter_mut().find(|a| a.id == id) {
            held.stacks = (held.stacks + 1).min(Self::MAX_STACKS);
        } else {
            self.augments.push(HeldAugment { id, stacks: 1 });
        }
        let after_stacks = self.stacks(id);
        AugmentGrantResult {
            id,
            before_stacks,
            after_stacks,
            reached_cap: after_stacks >= Self::MAX_STACKS,
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

    pub fn remove(&mut self, id: AugmentId) -> Option<HeldAugment> {
        let index = self.augments.iter().position(|held| held.id == id)?;
        Some(self.augments.remove(index))
    }

    pub fn remove_at(&mut self, index: usize) -> Option<HeldAugment> {
        (index < self.augments.len()).then(|| self.augments.remove(index))
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
        let result = inv.grant(AugmentId::Piercing);
        assert!(inv.has(AugmentId::Piercing));
        assert_eq!(inv.stacks(AugmentId::Piercing), 1);
        assert_eq!(result.before_stacks, 0);
        assert_eq!(result.after_stacks, 1);
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
        let capped = inv.grant(AugmentId::Piercing);
        let still_capped = inv.grant(AugmentId::Piercing);
        assert_eq!(inv.stacks(AugmentId::Piercing), 3);
        assert_eq!(capped.before_stacks, 2);
        assert_eq!(capped.after_stacks, 3);
        assert!(still_capped.reached_cap);
        assert_eq!(still_capped.after_stacks, 3);
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

    #[test]
    fn remove_returns_full_held_augment() {
        let mut inv = AugmentInventory::default();
        inv.add(AugmentId::Piercing);
        inv.add(AugmentId::Piercing);

        let removed = inv.remove(AugmentId::Piercing).expect("held augment");

        assert_eq!(removed.id, AugmentId::Piercing);
        assert_eq!(removed.stacks, 2);
        assert!(!inv.has(AugmentId::Piercing));
    }
}
