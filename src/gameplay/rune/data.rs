use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuneSlot {
    Melee,
    Ranged,
    Dash,
    Finisher,
}

impl RuneSlot {
    pub const ALL: [Self; 4] = [Self::Melee, Self::Ranged, Self::Dash, Self::Finisher];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuneTier {
    Common,
    Elite,
    Legendary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuneId {
    ImpactWave,
    SlowOnHit,
    ThirdStrikeExpand,
    WhirlSlash,
    ChainLightning,
    ExplosiveFist,
    VampireBlade,
    FrostTouch,
    PierceOne,
    MarkOnHit,
    RapidFireWeak,
    Scatter,
    HomingBullet,
    VenomShot,
    BarrageMode,
    DashEndShockwave,
    DashFirstCrit,
    Afterimage,
    ShadowClone,
    PhaseDash,
    BlinkDash,
    GroundSplitter,
    BoomerangBlade,
    DeathChain,
    WeaknessExpose,
    StormField,
    InstantThunder,
    PhoenixSoul,
    Berserker,
    ThornBody,
    EnergyShield,
}

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuneLoadout {
    pub melee: Option<RuneId>,
    pub ranged: Option<RuneId>,
    pub dash: Option<RuneId>,
    pub finisher: Option<RuneId>,
}

impl RuneLoadout {
    pub fn get(&self, slot: RuneSlot) -> Option<RuneId> {
        match slot {
            RuneSlot::Melee => self.melee,
            RuneSlot::Ranged => self.ranged,
            RuneSlot::Dash => self.dash,
            RuneSlot::Finisher => self.finisher,
        }
    }

    pub fn equip(&mut self, slot: RuneSlot, rune: RuneId) -> Option<RuneId> {
        let slot_ref = match slot {
            RuneSlot::Melee => &mut self.melee,
            RuneSlot::Ranged => &mut self.ranged,
            RuneSlot::Dash => &mut self.dash,
            RuneSlot::Finisher => &mut self.finisher,
        };
        let old = slot_ref.take();
        *slot_ref = Some(rune);
        old
    }
}
