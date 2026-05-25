//! Augment tuning lookup.
//!
//! All numerical values come from `assets/configs/augments.ron`; this module
//! is a thin typed wrapper around `GameDataRegistry::augment_param`. Whenever
//! a key is absent the helpers return a neutral value (0.0 / `None` / 1.0
//! where multiplicative). Adjust balance by editing the RON, not the code.

use super::data::AugmentId;
use crate::data::registry::GameDataRegistry;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HeavyStrikeProfile {
    pub(crate) damage_mult: f32,
    pub(crate) knockback_mult: f32,
    pub(crate) wall_damage_fraction: f32,
    pub(crate) stun_s: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ArmorBreakProfile {
    pub(crate) damage_multiplier: f32,
    pub(crate) duration_s: f32,
    pub(crate) crit_taken_bonus: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SwordWaveProfile {
    pub(crate) damage_fraction: f32,
    pub(crate) pierce_remaining: u8,
    pub(crate) full_energy_mult: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ChainLightningProfile {
    pub(crate) jumps: u8,
    pub(crate) damage_fraction: f32,
    pub(crate) paralyze_chance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ScatterProfile {
    pub(crate) shots: usize,
    pub(crate) damage_fraction: f32,
    pub(crate) ring: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FreezeProfile {
    pub(crate) chance: f32,
    pub(crate) duration_s: f32,
    pub(crate) shatter_bonus: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DashShieldProfile {
    pub(crate) charges: u8,
    pub(crate) cooldown_s: f32,
    pub(crate) break_damage_fraction: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlinkProfile {
    pub(crate) distance_mult: f32,
    pub(crate) impact_damage_fraction: f32,
    pub(crate) impact_radius_mult: f32,
    pub(crate) attack_bonus: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CritEnhanceProfile {
    pub(crate) crit_bonus: f32,
    pub(crate) crit_multiplier_bonus: f32,
    pub(crate) double_crit_chance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PhoenixProfile {
    pub(crate) revive_fraction: f32,
    pub(crate) invuln_s: f32,
    pub(crate) attack_bonus: f32,
}

fn param(data: &GameDataRegistry, id: AugmentId, stacks: u8, key: &str) -> f32 {
    data.augment_param_or(id, stacks, key, 0.0)
}

pub(crate) fn lifesteal_fraction(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::LifestealSlash, stacks, "lifesteal")
}

pub(crate) fn lifesteal_kill_heal(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::LifestealSlash, stacks, "kill_heal")
}

pub(crate) fn heavy_strike_profile(data: &GameDataRegistry, stacks: u8) -> HeavyStrikeProfile {
    let id = AugmentId::HeavyStrike;
    HeavyStrikeProfile {
        damage_mult: 1.0 + param(data, id, stacks, "damage"),
        knockback_mult: 1.0 + param(data, id, stacks, "knockback"),
        wall_damage_fraction: param(data, id, stacks, "wall_damage"),
        stun_s: param(data, id, stacks, "stun_s"),
    }
}

pub(crate) fn combo_accelerate_bonuses(
    data: &GameDataRegistry,
    stacks: u8,
    combo_count: u32,
) -> (f32, f32) {
    let id = AugmentId::ComboAccelerate;
    let threshold = data
        .augment_param(id, stacks, "threshold")
        .unwrap_or(f32::MAX);
    let attack_speed = param(data, id, stacks, "attack_speed");
    let crit_threshold = data
        .augment_param(id, stacks, "crit_threshold")
        .unwrap_or(f32::MAX);
    let crit = param(data, id, stacks, "crit");
    let speed = if (combo_count as f32) >= threshold {
        attack_speed
    } else {
        0.0
    };
    let crit_bonus = if (combo_count as f32) >= crit_threshold {
        crit
    } else {
        0.0
    };
    (speed, crit_bonus)
}

pub(crate) fn whirlwind_damage_mult(data: &GameDataRegistry, stacks: u8) -> f32 {
    let v = param(data, AugmentId::Whirlwind, stacks, "damage");
    if v == 0.0 { 1.0 } else { v }
}

pub(crate) fn whirlwind_range_mult(data: &GameDataRegistry, stacks: u8) -> f32 {
    1.0 + param(data, AugmentId::Whirlwind, stacks, "range")
}

pub(crate) fn armor_break_profile(
    data: &GameDataRegistry,
    stacks: u8,
) -> Option<ArmorBreakProfile> {
    let id = AugmentId::ArmorBreak;
    let vulnerability = data.augment_param(id, stacks, "vulnerability")?;
    Some(ArmorBreakProfile {
        damage_multiplier: 1.0 + vulnerability,
        duration_s: param(data, id, stacks, "duration_s"),
        crit_taken_bonus: param(data, id, stacks, "crit_taken"),
    })
}

pub(crate) fn reflect_damage_mult(data: &GameDataRegistry, stacks: u8) -> f32 {
    let v = param(data, AugmentId::Reflect, stacks, "damage");
    if v == 0.0 { 1.0 } else { v }
}

pub(crate) fn reflect_homing(data: &GameDataRegistry, stacks: u8) -> bool {
    param(data, AugmentId::Reflect, stacks, "homing") > 0.0
}

pub(crate) fn sword_wave_profile(
    data: &GameDataRegistry,
    stacks: u8,
    full_energy: bool,
) -> Option<SwordWaveProfile> {
    let id = AugmentId::SwordWave;
    let damage = data.augment_param(id, stacks, "damage")?;
    let full_energy_mult = data
        .augment_param(id, stacks, "full_energy_mult")
        .unwrap_or(1.0);
    let pierce = if param(data, id, stacks, "pierce") > 0.0 || stacks >= 3 {
        u8::MAX
    } else {
        0
    };
    let damage_fraction = if full_energy {
        damage * full_energy_mult
    } else {
        damage
    };
    Some(SwordWaveProfile {
        damage_fraction,
        pierce_remaining: pierce,
        full_energy_mult,
    })
}

pub(crate) fn executioner_threshold(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::Executioner, stacks, "threshold")
}

#[allow(dead_code)]
pub(crate) fn executioner_explosion_fraction(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::Executioner, stacks, "explode_max_hp")
}

pub(crate) fn charge_shot_damage_mult(data: &GameDataRegistry, stacks: u8) -> f32 {
    let v = param(data, AugmentId::Piercing, stacks, "damage");
    if v == 0.0 { 1.0 } else { 1.0 + v }
}

pub(crate) fn speed_boost_mult(data: &GameDataRegistry, stacks: u8) -> f32 {
    1.0 + param(data, AugmentId::SpeedBoost, stacks, "speed")
}

pub(crate) fn extra_projectile_count(data: &GameDataRegistry, stacks: u8) -> u8 {
    param(data, AugmentId::ExtraProjectile, stacks, "extra") as u8
}

/// Damage multiplier applied to each extra projectile spawned by the
/// ExtraProjectile augment. design.md §4.4 ranged: 60% / 75% / 100% per shot.
pub(crate) fn extra_projectile_damage_fraction(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::ExtraProjectile, stacks, "damage")
}

pub(crate) fn homing_turn_rate(data: &GameDataRegistry, stacks: u8) -> f32 {
    // Dimensionless Vec2::lerp factor (0..1). Consumed by
    // homing_projectile_system as `current_dir.lerp(target_dir, factor)`,
    // so unit is "fraction of the way to the target this frame", NOT radians.
    param(data, AugmentId::Homing, stacks, "turn_factor")
}

pub(crate) fn homing_search_radius(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::Homing, stacks, "search_radius")
}

pub(crate) fn homing_snap_radius(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::Homing, stacks, "snap_radius")
}

pub(crate) fn homing_pierce(data: &GameDataRegistry, stacks: u8) -> u8 {
    param(data, AugmentId::Homing, stacks, "pierce") as u8
}

pub(crate) fn chain_lightning_profile(
    data: &GameDataRegistry,
    stacks: u8,
) -> Option<ChainLightningProfile> {
    let id = AugmentId::ChainLightning;
    let jumps = data.augment_param(id, stacks, "jumps")? as u8;
    Some(ChainLightningProfile {
        jumps,
        damage_fraction: param(data, id, stacks, "damage"),
        paralyze_chance: param(data, id, stacks, "paralyze"),
    })
}

pub(crate) fn scatter_profile(data: &GameDataRegistry, stacks: u8) -> Option<ScatterProfile> {
    let id = AugmentId::Scatter;
    let shots = data.augment_param(id, stacks, "shots")? as usize;
    Some(ScatterProfile {
        shots,
        damage_fraction: param(data, id, stacks, "damage"),
        ring: param(data, id, stacks, "ring") > 0.0,
    })
}

pub(crate) fn bullet_storm_projectile_count(data: &GameDataRegistry, stacks: u8) -> usize {
    let id = AugmentId::BulletStorm;
    let directions = data.augment_param(id, stacks, "directions").unwrap_or(0.0);
    let waves = data.augment_param(id, stacks, "waves").unwrap_or(0.0);
    (directions * waves) as usize
}

#[allow(dead_code)]
pub(crate) fn bullet_storm_energy_on_hit(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::BulletStorm, stacks, "energy_on_hit")
}

pub(crate) fn freeze_profile(data: &GameDataRegistry, stacks: u8) -> Option<FreezeProfile> {
    let id = AugmentId::Freeze;
    let chance = data.augment_param(id, stacks, "chance")?;
    Some(FreezeProfile {
        chance,
        duration_s: param(data, id, stacks, "duration_s"),
        shatter_bonus: param(data, id, stacks, "shatter"),
    })
}

pub(crate) fn dash_trail_damage_fraction(data: &GameDataRegistry, stacks: u8) -> Option<f32> {
    data.augment_param(AugmentId::DashTrail, stacks, "damage")
}

pub(crate) fn dash_energy_gain(
    data: &GameDataRegistry,
    stacks: u8,
    unique_hits_after_hit: usize,
) -> f32 {
    let id = AugmentId::DashEnergy;
    let base = param(data, id, stacks, "energy");
    let bonus_threshold = data.augment_param(id, stacks, "bonus_threshold");
    let bonus_energy = param(data, id, stacks, "bonus_energy");
    if let Some(threshold) = bonus_threshold
        && unique_hits_after_hit as f32 >= threshold
    {
        base + bonus_energy
    } else {
        base
    }
}

pub(crate) fn extended_invuln_bonus(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::ExtendedInvuln, stacks, "duration_s")
}

#[allow(dead_code)]
pub(crate) fn extended_invuln_contact_damage_fraction(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::ExtendedInvuln, stacks, "contact_damage")
}

#[allow(dead_code)]
pub(crate) fn dash_reset_damage_fraction(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::DashReset, stacks, "damage")
}

pub(crate) fn dash_shield_profile(
    data: &GameDataRegistry,
    stacks: u8,
) -> Option<DashShieldProfile> {
    let id = AugmentId::DashShield;
    let charges = data.augment_param(id, stacks, "charges")? as u8;
    Some(DashShieldProfile {
        charges,
        cooldown_s: param(data, id, stacks, "cooldown_s"),
        break_damage_fraction: param(data, id, stacks, "break_damage"),
    })
}

pub(crate) fn blink_profile(data: &GameDataRegistry, stacks: u8) -> Option<BlinkProfile> {
    let id = AugmentId::Blink;
    let damage = data.augment_param(id, stacks, "damage")?;
    Some(BlinkProfile {
        distance_mult: 1.0 + data.augment_param(id, stacks, "distance").unwrap_or(0.0),
        impact_damage_fraction: damage,
        impact_radius_mult: 1.0 + data.augment_param(id, stacks, "range").unwrap_or(0.0),
        attack_bonus: param(data, id, stacks, "attack_bonus"),
    })
}

pub(crate) fn gold_bonus_mult(data: &GameDataRegistry, stacks: u8) -> f32 {
    1.0 + param(data, AugmentId::GoldBonus, stacks, "gold")
}

pub(crate) fn xp_bonus_mult(data: &GameDataRegistry, stacks: u8) -> f32 {
    1.0 + param(data, AugmentId::XpBonus, stacks, "xp")
}

pub(crate) fn pickup_range_mult(data: &GameDataRegistry, stacks: u8) -> f32 {
    1.0 + param(data, AugmentId::PickupRange, stacks, "range")
}

pub(crate) fn thorns_damage_fraction(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::Thorns, stacks, "damage")
}

pub(crate) fn kill_heal_amount(data: &GameDataRegistry, stacks: u8) -> f32 {
    param(data, AugmentId::KillHeal, stacks, "heal")
}

pub(crate) fn crit_enhance_profile(data: &GameDataRegistry, stacks: u8) -> CritEnhanceProfile {
    let id = AugmentId::CritEnhance;
    CritEnhanceProfile {
        crit_bonus: param(data, id, stacks, "crit"),
        crit_multiplier_bonus: param(data, id, stacks, "crit_damage"),
        double_crit_chance: param(data, id, stacks, "double_crit"),
    }
}

pub(crate) fn phoenix_profile(data: &GameDataRegistry, stacks: u8) -> Option<PhoenixProfile> {
    let id = AugmentId::Phoenix;
    let revive = data.augment_param(id, stacks, "revive")?;
    Some(PhoenixProfile {
        revive_fraction: revive,
        invuln_s: param(data, id, stacks, "invuln_s"),
        attack_bonus: param(data, id, stacks, "attack_bonus"),
    })
}

pub(crate) fn greed_damage_mult(data: &GameDataRegistry, stacks: u8, gold: u32) -> f32 {
    let id = AugmentId::Greed;
    let threshold = data.augment_param(id, stacks, "gold_threshold");
    let per_step = param(data, id, stacks, "damage");
    match threshold {
        Some(t) if t > 0.0 => 1.0 + (gold as f32 / t).floor() * per_step,
        _ => 1.0,
    }
}

pub(crate) fn greed_crit_bonus(data: &GameDataRegistry, stacks: u8, gold: u32) -> f32 {
    let id = AugmentId::Greed;
    let threshold = data.augment_param(id, stacks, "crit_gold_threshold");
    let per_step = param(data, id, stacks, "crit");
    match threshold {
        Some(t) if t > 0.0 => (gold as f32 / t).floor() * per_step,
        _ => 0.0,
    }
}
