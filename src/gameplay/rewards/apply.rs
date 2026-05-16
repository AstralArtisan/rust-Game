use crate::data::definitions::RewardScalingConfig;
use crate::gameplay::player::components::{
    AttackCooldown, AttackPower, CritChance, DashCooldown, Health, MoveSpeed, RangedCooldown,
    RewardModifiers,
};
use crate::gameplay::rewards::data::RewardType;

pub fn apply_reward_to_player_components(
    reward: RewardType,
    floor_number: u32,
    reward_scale: f32,
    scaling: &RewardScalingConfig,
    mods: &mut RewardModifiers,
    health: &mut Health,
    move_speed: &mut MoveSpeed,
    dash_cd: &mut DashCooldown,
    ranged_cd: &mut RangedCooldown,
    crit: &mut CritChance,
    atk_cd: &mut AttackCooldown,
    attack_power: &mut AttackPower,
) {
    match reward {
        RewardType::RecoverHealth => {
            let heal = heal_amount(scaling, health.max, floor_number) * reward_scale.max(1.0);
            health.current = (health.current + heal).min(health.max);
        }
        RewardType::EnhanceMeleeWeapon => {
            mods.melee_mastery_stacks = (mods.melee_mastery_stacks + 1)
                .min(RewardModifiers::WEAPON_MASTERY_MAX_LEVEL as u32);
        }
        RewardType::IncreaseAttackSpeed => {
            if !mods.reward_at_max(reward) {
                mods.attack_speed_level += 1;
                let remain =
                    (RewardModifiers::ATTACK_SPEED_CAP_S - mods.attack_speed_reduction_s).max(0.0);
                mods.attack_speed_reduction_s +=
                    (attack_speed_gain_s(scaling, floor_number) * reward_scale).min(remain);
            }
        }
        RewardType::IncreaseAttackPower => {
            if !mods.reward_at_max(reward) {
                mods.attack_power_level += 1;
                attack_power.0 += attack_power_gain(scaling, floor_number) * reward_scale;
            }
        }
        RewardType::IncreaseMaxHealth => {
            if !mods.reward_at_max(reward) {
                mods.max_health_level += 1;
                let gain = max_health_gain(scaling, floor_number) * reward_scale;
                mods.max_hp_add += gain;
                health.max += gain;
                health.current = (health.current + gain).min(health.max);
            }
        }
        RewardType::ReduceDashCooldown => {
            if !mods.reward_at_max(reward) {
                mods.dash_cooldown_level += 1;
                let remain = (RewardModifiers::DASH_COOLDOWN_CAP_S
                    - mods.dash_cooldown_reduction_s)
                    .max(0.0);
                mods.dash_cooldown_reduction_s +=
                    (dash_cooldown_gain_s(scaling, floor_number) * reward_scale).min(remain);
            }
        }
        RewardType::LifeStealOnKill => {
            if !mods.reward_at_max(reward) {
                mods.lifesteal_level += 1;
                mods.lifesteal_on_kill += lifesteal_gain(scaling, floor_number) * reward_scale;
            }
        }
        RewardType::IncreaseCritChance => {
            if !mods.reward_at_max(reward) {
                mods.crit_level += 1;
                let gain = crit_gain(scaling, floor_number) * reward_scale;
                mods.crit_add += gain;
                crit.0 = (crit.0 + gain).clamp(0.0, 1.0);
            }
        }
        RewardType::IncreaseMoveSpeed => {
            if !mods.reward_at_max(reward) {
                mods.move_speed_level += 1;
                let gain = move_speed_gain(scaling, floor_number) * reward_scale;
                mods.move_speed_add += gain;
                move_speed.0 += gain;
            }
        }
        RewardType::DashDamageTrail => mods.dash_damage_trail = true,
        RewardType::EnhanceRangedWeapon => {
            mods.ranged_mastery_stacks = (mods.ranged_mastery_stacks + 1)
                .min(RewardModifiers::WEAPON_MASTERY_MAX_LEVEL as u32);
        }
    }

    dash_cd.apply_reduction(mods.total_dash_cooldown_reduction());
    atk_cd.apply_speed_bonus(mods.total_melee_speed_bonus());
    ranged_cd.apply_speed_bonus(mods.total_ranged_speed_bonus());
}

pub fn attack_speed_gain_s(scaling: &RewardScalingConfig, floor_number: u32) -> f32 {
    scaling.attack_speed_s.get(floor_number)
}

pub fn attack_power_gain(scaling: &RewardScalingConfig, floor_number: u32) -> f32 {
    scaling.attack_power.get(floor_number)
}

pub fn max_health_gain(scaling: &RewardScalingConfig, floor_number: u32) -> f32 {
    scaling.max_health.get(floor_number)
}

pub fn dash_cooldown_gain_s(scaling: &RewardScalingConfig, floor_number: u32) -> f32 {
    scaling.dash_cooldown_s.get(floor_number)
}

pub fn lifesteal_gain(scaling: &RewardScalingConfig, floor_number: u32) -> f32 {
    scaling.lifesteal.get(floor_number)
}

pub fn crit_gain(scaling: &RewardScalingConfig, floor_number: u32) -> f32 {
    scaling.crit_chance.get(floor_number)
}

pub fn move_speed_gain(scaling: &RewardScalingConfig, floor_number: u32) -> f32 {
    scaling.move_speed.get(floor_number)
}

pub fn heal_amount(scaling: &RewardScalingConfig, max_hp: f32, floor_number: u32) -> f32 {
    let flat = scaling.heal_base.get(floor_number);
    (max_hp * scaling.heal_hp_fraction).max(flat)
}
