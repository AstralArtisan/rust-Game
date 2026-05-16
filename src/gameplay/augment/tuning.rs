#[cfg(test)]
use super::data::AugmentId;

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

fn capped(stacks: u8) -> u8 {
    stacks.min(3)
}

pub(crate) fn lifesteal_fraction(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 0.08,
        2 => 0.05,
        1 => 0.03,
        _ => 0.0,
    }
}

pub(crate) fn lifesteal_kill_heal(stacks: u8) -> f32 {
    if capped(stacks) >= 3 { 15.0 } else { 0.0 }
}

pub(crate) fn heavy_strike_profile(stacks: u8) -> HeavyStrikeProfile {
    match capped(stacks) {
        3 => HeavyStrikeProfile {
            damage_mult: 1.30,
            knockback_mult: 2.50,
            wall_damage_fraction: 0.50,
            stun_s: 0.50,
        },
        2 => HeavyStrikeProfile {
            damage_mult: 1.20,
            knockback_mult: 2.00,
            wall_damage_fraction: 0.0,
            stun_s: 0.0,
        },
        1 => HeavyStrikeProfile {
            damage_mult: 1.10,
            knockback_mult: 1.60,
            wall_damage_fraction: 0.0,
            stun_s: 0.0,
        },
        _ => HeavyStrikeProfile {
            damage_mult: 1.0,
            knockback_mult: 1.0,
            wall_damage_fraction: 0.0,
            stun_s: 0.0,
        },
    }
}

pub(crate) fn combo_accelerate_bonuses(stacks: u8, combo_count: u32) -> (f32, f32) {
    match capped(stacks) {
        3 if combo_count >= 10 => (0.50, 0.15),
        3 if combo_count >= 3 => (0.50, 0.0),
        2 if combo_count >= 3 => (0.35, 0.0),
        1 if combo_count >= 5 => (0.20, 0.0),
        _ => (0.0, 0.0),
    }
}

pub(crate) fn whirlwind_damage_mult(stacks: u8) -> f32 {
    match capped(stacks) {
        3 | 2 => 1.10,
        1 => 1.0,
        _ => 1.0,
    }
}

pub(crate) fn whirlwind_range_mult(stacks: u8) -> f32 {
    if capped(stacks) >= 2 { 1.30 } else { 1.0 }
}

pub(crate) fn armor_break_profile(stacks: u8) -> Option<ArmorBreakProfile> {
    match capped(stacks) {
        3 => Some(ArmorBreakProfile {
            damage_multiplier: 1.35,
            duration_s: 5.0,
            crit_taken_bonus: 0.20,
        }),
        2 => Some(ArmorBreakProfile {
            damage_multiplier: 1.25,
            duration_s: 4.0,
            crit_taken_bonus: 0.0,
        }),
        1 => Some(ArmorBreakProfile {
            damage_multiplier: 1.15,
            duration_s: 3.0,
            crit_taken_bonus: 0.0,
        }),
        _ => None,
    }
}

pub(crate) fn reflect_damage_mult(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 2.0,
        2 => 1.5,
        1 => 1.0,
        _ => 1.10,
    }
}

pub(crate) fn reflect_homing(stacks: u8) -> bool {
    capped(stacks) >= 3
}

pub(crate) fn sword_wave_profile(stacks: u8, full_energy: bool) -> Option<SwordWaveProfile> {
    let mut profile = match capped(stacks) {
        3 => SwordWaveProfile {
            damage_fraction: 0.70,
            pierce_remaining: 255,
            full_energy_mult: 2.0,
        },
        2 => SwordWaveProfile {
            damage_fraction: 0.55,
            pierce_remaining: 255,
            full_energy_mult: 1.0,
        },
        1 => SwordWaveProfile {
            damage_fraction: 0.40,
            pierce_remaining: 0,
            full_energy_mult: 1.0,
        },
        _ => return None,
    };
    if full_energy {
        profile.damage_fraction *= profile.full_energy_mult;
    }
    Some(profile)
}

pub(crate) fn executioner_threshold(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 0.20,
        2 => 0.15,
        1 => 0.10,
        _ => 0.0,
    }
}

#[allow(dead_code)]
pub(crate) fn executioner_explosion_fraction(stacks: u8) -> f32 {
    if capped(stacks) >= 3 { 0.30 } else { 0.0 }
}

pub(crate) fn charge_shot_damage_mult(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 2.20,
        2 => 2.20,
        1 => 1.80,
        _ => 1.0,
    }
}

pub(crate) fn speed_boost_mult(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 1.70,
        2 => 1.50,
        1 => 1.30,
        _ => 1.0,
    }
}

pub(crate) fn extra_projectile_count(stacks: u8) -> u8 {
    capped(stacks)
}

pub(crate) fn homing_turn_rate(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 0.42,
        2 => 0.22,
        1 => 0.12,
        _ => 0.0,
    }
}

pub(crate) fn homing_search_radius(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 420.0,
        2 => 320.0,
        1 => 240.0,
        _ => 0.0,
    }
}

pub(crate) fn homing_pierce(stacks: u8) -> u8 {
    if capped(stacks) >= 3 { 1 } else { 0 }
}

pub(crate) fn chain_lightning_profile(stacks: u8) -> Option<ChainLightningProfile> {
    match capped(stacks) {
        3 => Some(ChainLightningProfile {
            jumps: 3,
            damage_fraction: 0.70,
            paralyze_chance: 0.20,
        }),
        2 => Some(ChainLightningProfile {
            jumps: 2,
            damage_fraction: 0.60,
            paralyze_chance: 0.0,
        }),
        1 => Some(ChainLightningProfile {
            jumps: 1,
            damage_fraction: 0.50,
            paralyze_chance: 0.0,
        }),
        _ => None,
    }
}

pub(crate) fn scatter_profile(stacks: u8) -> Option<ScatterProfile> {
    match capped(stacks) {
        3 => Some(ScatterProfile {
            shots: 8,
            damage_fraction: 0.90,
            ring: true,
        }),
        2 => Some(ScatterProfile {
            shots: 5,
            damage_fraction: 0.85,
            ring: false,
        }),
        1 => Some(ScatterProfile {
            shots: 3,
            damage_fraction: 0.80,
            ring: false,
        }),
        _ => None,
    }
}

pub(crate) fn bullet_storm_projectile_count(stacks: u8) -> usize {
    match capped(stacks) {
        3 => 36,
        2 => 16,
        1 => 6,
        _ => 0,
    }
}

#[allow(dead_code)]
pub(crate) fn bullet_storm_energy_on_hit(stacks: u8) -> f32 {
    if capped(stacks) >= 3 { 2.0 } else { 0.0 }
}

pub(crate) fn freeze_profile(stacks: u8) -> Option<FreezeProfile> {
    match capped(stacks) {
        3 => Some(FreezeProfile {
            chance: 0.25,
            duration_s: 2.0,
            shatter_bonus: 0.50,
        }),
        2 => Some(FreezeProfile {
            chance: 0.18,
            duration_s: 1.5,
            shatter_bonus: 0.0,
        }),
        1 => Some(FreezeProfile {
            chance: 0.12,
            duration_s: 1.0,
            shatter_bonus: 0.0,
        }),
        _ => None,
    }
}

pub(crate) fn dash_trail_damage_fraction(stacks: u8) -> Option<f32> {
    match capped(stacks) {
        3 => Some(1.00),
        2 => Some(0.80),
        1 => Some(0.50),
        _ => None,
    }
}

pub(crate) fn dash_energy_gain(stacks: u8, unique_hits_after_hit: usize) -> f32 {
    let base = match capped(stacks) {
        3 => 20.0,
        2 => 15.0,
        1 => 10.0,
        _ => 0.0,
    };
    if capped(stacks) >= 3 && unique_hits_after_hit == 3 {
        base + 15.0
    } else {
        base
    }
}

pub(crate) fn extended_invuln_bonus(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 0.35,
        2 => 0.25,
        1 => 0.15,
        _ => 0.0,
    }
}

#[allow(dead_code)]
pub(crate) fn extended_invuln_contact_damage_fraction(stacks: u8) -> f32 {
    if capped(stacks) >= 3 { 0.30 } else { 0.0 }
}

#[allow(dead_code)]
pub(crate) fn dash_reset_damage_fraction(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 1.50,
        2 => 1.00,
        1 => 0.60,
        _ => 0.0,
    }
}

pub(crate) fn dash_shield_profile(stacks: u8) -> Option<DashShieldProfile> {
    match capped(stacks) {
        3 => Some(DashShieldProfile {
            charges: 3,
            cooldown_s: 5.0,
            break_damage_fraction: 0.80,
        }),
        2 => Some(DashShieldProfile {
            charges: 2,
            cooldown_s: 6.0,
            break_damage_fraction: 0.0,
        }),
        1 => Some(DashShieldProfile {
            charges: 1,
            cooldown_s: 8.0,
            break_damage_fraction: 0.0,
        }),
        _ => None,
    }
}

pub(crate) fn blink_profile(stacks: u8) -> Option<BlinkProfile> {
    match capped(stacks) {
        3 => Some(BlinkProfile {
            distance_mult: 1.50,
            impact_damage_fraction: 1.20,
            impact_radius_mult: 1.50,
            attack_bonus: 0.30,
        }),
        2 => Some(BlinkProfile {
            distance_mult: 1.0,
            impact_damage_fraction: 0.80,
            impact_radius_mult: 1.50,
            attack_bonus: 0.0,
        }),
        1 => Some(BlinkProfile {
            distance_mult: 1.0,
            impact_damage_fraction: 0.50,
            impact_radius_mult: 1.0,
            attack_bonus: 0.0,
        }),
        _ => None,
    }
}

pub(crate) fn gold_bonus_mult(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 1.60,
        2 => 1.40,
        1 => 1.20,
        _ => 1.0,
    }
}

pub(crate) fn xp_bonus_mult(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 1.60,
        2 => 1.40,
        1 => 1.20,
        _ => 1.0,
    }
}

pub(crate) fn pickup_range_mult(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 2.20,
        2 => 1.80,
        1 => 1.50,
        _ => 1.0,
    }
}

pub(crate) fn thorns_damage_fraction(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 0.50,
        2 => 0.35,
        1 => 0.20,
        _ => 0.0,
    }
}

pub(crate) fn kill_heal_amount(stacks: u8) -> f32 {
    match capped(stacks) {
        3 => 12.0,
        2 => 8.0,
        1 => 5.0,
        _ => 0.0,
    }
}

pub(crate) fn crit_enhance_profile(stacks: u8) -> CritEnhanceProfile {
    match capped(stacks) {
        3 => CritEnhanceProfile {
            crit_bonus: 0.08,
            crit_multiplier_bonus: 0.35,
            double_crit_chance: 0.15,
        },
        2 => CritEnhanceProfile {
            crit_bonus: 0.0,
            crit_multiplier_bonus: 0.35,
            double_crit_chance: 0.0,
        },
        1 => CritEnhanceProfile {
            crit_bonus: 0.08,
            crit_multiplier_bonus: 0.0,
            double_crit_chance: 0.0,
        },
        _ => CritEnhanceProfile {
            crit_bonus: 0.0,
            crit_multiplier_bonus: 0.0,
            double_crit_chance: 0.0,
        },
    }
}

pub(crate) fn phoenix_profile(stacks: u8) -> Option<PhoenixProfile> {
    match capped(stacks) {
        3 => Some(PhoenixProfile {
            revive_fraction: 1.0,
            invuln_s: 3.0,
            attack_bonus: 0.30,
        }),
        2 => Some(PhoenixProfile {
            revive_fraction: 0.70,
            invuln_s: 0.0,
            attack_bonus: 0.0,
        }),
        1 => Some(PhoenixProfile {
            revive_fraction: 0.50,
            invuln_s: 0.0,
            attack_bonus: 0.0,
        }),
        _ => None,
    }
}

pub(crate) fn greed_damage_mult(stacks: u8, gold: u32) -> f32 {
    match capped(stacks) {
        3 => 1.0 + (gold / 60) as f32 * 0.04,
        2 => 1.0 + (gold / 80) as f32 * 0.03,
        1 => 1.0 + (gold / 100) as f32 * 0.03,
        _ => 1.0,
    }
}

pub(crate) fn greed_crit_bonus(stacks: u8, gold: u32) -> f32 {
    if capped(stacks) >= 3 {
        (gold / 200) as f32 * 0.01
    } else {
        0.0
    }
}

#[cfg(test)]
pub(crate) fn assert_known_augment_id(id: AugmentId) {
    match id {
        AugmentId::LifestealSlash
        | AugmentId::HeavyStrike
        | AugmentId::ComboAccelerate
        | AugmentId::Whirlwind
        | AugmentId::ArmorBreak
        | AugmentId::Reflect
        | AugmentId::SwordWave
        | AugmentId::Executioner
        | AugmentId::Piercing
        | AugmentId::SpeedBoost
        | AugmentId::ExtraProjectile
        | AugmentId::Homing
        | AugmentId::ChainLightning
        | AugmentId::Scatter
        | AugmentId::BulletStorm
        | AugmentId::Freeze
        | AugmentId::DashTrail
        | AugmentId::DashEnergy
        | AugmentId::ExtendedInvuln
        | AugmentId::DashReset
        | AugmentId::DashShield
        | AugmentId::Blink
        | AugmentId::GoldBonus
        | AugmentId::XpBonus
        | AugmentId::PickupRange
        | AugmentId::Thorns
        | AugmentId::KillHeal
        | AugmentId::CritEnhance
        | AugmentId::Phoenix
        | AugmentId::Greed => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn phase3_tuning_covers_all_augment_ids() {
        for id in [
            AugmentId::LifestealSlash,
            AugmentId::HeavyStrike,
            AugmentId::ComboAccelerate,
            AugmentId::Whirlwind,
            AugmentId::ArmorBreak,
            AugmentId::Reflect,
            AugmentId::SwordWave,
            AugmentId::Executioner,
            AugmentId::Piercing,
            AugmentId::SpeedBoost,
            AugmentId::ExtraProjectile,
            AugmentId::Homing,
            AugmentId::ChainLightning,
            AugmentId::Scatter,
            AugmentId::BulletStorm,
            AugmentId::Freeze,
            AugmentId::DashTrail,
            AugmentId::DashEnergy,
            AugmentId::ExtendedInvuln,
            AugmentId::DashReset,
            AugmentId::DashShield,
            AugmentId::Blink,
            AugmentId::GoldBonus,
            AugmentId::XpBonus,
            AugmentId::PickupRange,
            AugmentId::Thorns,
            AugmentId::KillHeal,
            AugmentId::CritEnhance,
            AugmentId::Phoenix,
            AugmentId::Greed,
        ] {
            assert_known_augment_id(id);
        }
    }

    #[test]
    fn melee_profiles_match_phase3_spec() {
        close(lifesteal_fraction(1), 0.03);
        close(lifesteal_fraction(2), 0.05);
        close(lifesteal_fraction(3), 0.08);
        close(lifesteal_kill_heal(3), 15.0);

        close(heavy_strike_profile(1).damage_mult, 1.10);
        close(heavy_strike_profile(2).knockback_mult, 2.00);
        close(heavy_strike_profile(3).wall_damage_fraction, 0.50);
        assert_eq!(combo_accelerate_bonuses(3, 10), (0.50, 0.15));
        close(whirlwind_damage_mult(2), 1.10);
        close(armor_break_profile(3).unwrap().damage_multiplier, 1.35);
        close(reflect_damage_mult(3), 2.0);
        assert!(reflect_homing(3));
        close(sword_wave_profile(3, true).unwrap().damage_fraction, 1.40);
        close(executioner_threshold(3), 0.20);
        close(executioner_explosion_fraction(3), 0.30);
    }

    #[test]
    fn ranged_profiles_match_phase3_spec() {
        close(charge_shot_damage_mult(1), 1.80);
        close(speed_boost_mult(3), 1.70);
        assert_eq!(extra_projectile_count(3), 3);
        close(homing_turn_rate(3), 0.42);
        assert_eq!(homing_pierce(3), 1);
        let chain = chain_lightning_profile(3).unwrap();
        assert_eq!(chain.jumps, 3);
        close(chain.damage_fraction, 0.70);
        let scatter = scatter_profile(3).unwrap();
        assert_eq!(scatter.shots, 8);
        assert!(scatter.ring);
        assert_eq!(bullet_storm_projectile_count(3), 36);
        close(bullet_storm_energy_on_hit(3), 2.0);
        let freeze = freeze_profile(3).unwrap();
        close(freeze.chance, 0.25);
        close(freeze.shatter_bonus, 0.50);
    }

    #[test]
    fn mobility_and_general_profiles_match_phase3_spec() {
        close(dash_trail_damage_fraction(3).unwrap(), 1.0);
        close(dash_energy_gain(3, 3), 35.0);
        close(extended_invuln_bonus(3), 0.35);
        close(extended_invuln_contact_damage_fraction(3), 0.30);
        close(dash_reset_damage_fraction(3), 1.50);
        assert_eq!(dash_shield_profile(3).unwrap().charges, 3);
        close(blink_profile(3).unwrap().attack_bonus, 0.30);
        close(gold_bonus_mult(3), 1.60);
        close(xp_bonus_mult(3), 1.60);
        close(pickup_range_mult(3), 2.20);
        close(thorns_damage_fraction(3), 0.50);
        close(kill_heal_amount(3), 12.0);
        let crit = crit_enhance_profile(3);
        close(crit.crit_bonus, 0.08);
        close(crit.crit_multiplier_bonus, 0.35);
        close(crit.double_crit_chance, 0.15);
        close(phoenix_profile(3).unwrap().revive_fraction, 1.0);
        close(greed_damage_mult(3, 180), 1.12);
        close(greed_crit_bonus(3, 400), 0.02);
    }
}
