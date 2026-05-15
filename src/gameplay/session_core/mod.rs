use crate::data::definitions::RewardScalingConfig;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentRarity};
use crate::gameplay::map::room::RoomType;
use crate::gameplay::player::components::{
    AttackCooldown, AttackPower, CritChance, DashCooldown, ENERGY_SYSTEM_ENABLED, Energy, Health,
    MoveSpeed, RangedCooldown, RewardModifiers,
};
use crate::gameplay::rewards::apply::{
    attack_power_gain, attack_speed_gain_s, crit_gain, dash_cooldown_gain_s, max_health_gain,
    move_speed_gain,
};
use crate::utils::rng::GameRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    Solo,
    #[allow(dead_code)]
    Coop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionRuleContext {
    pub mode: SessionMode,
    pub floor: u32,
    pub total_floors: u32,
    pub boss_gives_victory: bool,
    pub room_type: RoomType,
}

#[derive(Debug)]
pub struct PlayerRuleEffects<'a> {
    pub health: &'a mut Health,
    pub energy: &'a mut Energy,
    pub move_speed: &'a mut MoveSpeed,
    pub attack_power: &'a mut AttackPower,
    pub crit: &'a mut CritChance,
    pub dash_cooldown: &'a mut DashCooldown,
    pub attack_cooldown: &'a mut AttackCooldown,
    pub ranged_cooldown: &'a mut RangedCooldown,
    pub mods: &'a mut RewardModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostRewardDecision {
    ResumeRun,
    NextFloor,
    Victory,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomClearDecision {
    pub heal_alive_fraction: f32,
    pub post_reward: PostRewardDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathDecision {
    Continue,
    GameOver,
    MatchOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedShopItem {
    Heal,
    IncreaseMaxHealth,
    IncreaseAttackPower,
    ReduceDashCooldown,
    IncreaseMoveSpeed,
    IncreaseEnergyMax,
    IncreaseCritChance,
    IncreaseAttackSpeed,
    Augment(AugmentId),
    HealingPotion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopOfferDraft {
    pub item: SharedShopItem,
    pub cost: u32,
    pub purchased: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopDraft {
    pub refresh_count: u32,
    pub offers: Vec<ShopOfferDraft>,
    pub augment_offers: Vec<ShopOfferDraft>,
    pub utility_offers: Vec<ShopOfferDraft>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopPurchaseResult {
    Applied,
    NoEffect,
}

pub fn on_room_cleared(ctx: SessionRuleContext) -> RoomClearDecision {
    match ctx.room_type {
        RoomType::Boss => {
            let reached_final_floor = ctx.floor >= ctx.total_floors.max(1);
            let post_reward = if ctx.boss_gives_victory || reached_final_floor {
                PostRewardDecision::Victory
            } else {
                PostRewardDecision::NextFloor
            };
            RoomClearDecision {
                heal_alive_fraction: 0.80,
                post_reward,
            }
        }
        _ => RoomClearDecision {
            heal_alive_fraction: 0.0,
            post_reward: PostRewardDecision::ResumeRun,
        },
    }
}

pub fn build_shop_draft(
    floor_number: u32,
    mods: RewardModifiers,
    rng: &mut GameRng,
    registry: Option<&GameDataRegistry>,
) -> ShopDraft {
    ShopDraft {
        refresh_count: 0,
        offers: build_shop_offers(floor_number, mods, rng),
        augment_offers: registry
            .map(|registry| build_augment_offers(registry, rng, floor_number))
            .unwrap_or_default(),
        utility_offers: build_utility_offers(),
    }
}

pub fn refresh_shop_draft(
    refresh_count: u32,
    floor_number: u32,
    mods: RewardModifiers,
    rng: &mut GameRng,
    registry: Option<&GameDataRegistry>,
) -> ShopDraft {
    ShopDraft {
        refresh_count: refresh_count.saturating_add(1),
        offers: build_shop_offers(floor_number, mods, rng),
        augment_offers: registry
            .map(|registry| build_augment_offers(registry, rng, floor_number))
            .unwrap_or_default(),
        utility_offers: build_utility_offers(),
    }
}

pub fn apply_shop_purchase(
    item: SharedShopItem,
    floor_number: u32,
    effects: &mut PlayerRuleEffects<'_>,
    scaling: &RewardScalingConfig,
) -> ShopPurchaseResult {
    if apply_shop_item(item, floor_number, effects, scaling) {
        ShopPurchaseResult::Applied
    } else {
        ShopPurchaseResult::NoEffect
    }
}

pub fn evaluate_death(mode: SessionMode, living_players: usize) -> DeathDecision {
    if living_players > 0 {
        return DeathDecision::Continue;
    }

    match mode {
        SessionMode::Solo => DeathDecision::GameOver,
        SessionMode::Coop => DeathDecision::MatchOver,
    }
}

pub fn next_refresh_cost(refresh_count: u32) -> u32 {
    if refresh_count == 0 {
        0
    } else {
        refresh_count.saturating_mul(50)
    }
}

fn build_shop_offers(
    floor_number: u32,
    mods: RewardModifiers,
    rng: &mut GameRng,
) -> Vec<ShopOfferDraft> {
    let mut pool = vec![
        SharedShopItem::Heal,
        SharedShopItem::IncreaseMaxHealth,
        SharedShopItem::IncreaseAttackPower,
        SharedShopItem::ReduceDashCooldown,
        SharedShopItem::IncreaseMoveSpeed,
        SharedShopItem::IncreaseEnergyMax,
        SharedShopItem::IncreaseCritChance,
        SharedShopItem::IncreaseAttackSpeed,
    ];
    if !ENERGY_SYSTEM_ENABLED {
        pool.retain(|item| *item != SharedShopItem::IncreaseEnergyMax);
    }
    rng.shuffle(&mut pool);
    pool.truncate(3);

    let base_cost = shop_base_cost(floor_number);
    pool.into_iter()
        .map(|item| ShopOfferDraft {
            item,
            cost: shop_item_cost(item, base_cost, mods),
            purchased: false,
        })
        .collect()
}

fn build_augment_offers(
    registry: &GameDataRegistry,
    rng: &mut GameRng,
    floor_number: u32,
) -> Vec<ShopOfferDraft> {
    let target_count = if floor_number >= 3 { 3 } else { 2 };
    let mut pool = registry.augments.augments.iter().collect::<Vec<_>>();
    if pool.is_empty() {
        return Vec::new();
    }

    rng.shuffle(&mut pool);
    pool.truncate(target_count.min(pool.len()));

    pool.into_iter()
        .map(|augment| ShopOfferDraft {
            item: SharedShopItem::Augment(augment.id),
            cost: augment_shop_cost(augment.shop_cost, augment.rarity),
            purchased: false,
        })
        .collect()
}

fn build_utility_offers() -> Vec<ShopOfferDraft> {
    vec![ShopOfferDraft {
        item: SharedShopItem::HealingPotion,
        cost: 30,
        purchased: false,
    }]
}

fn augment_shop_cost(shop_cost: u32, rarity: AugmentRarity) -> u32 {
    if shop_cost > 0 {
        shop_cost
    } else {
        match rarity {
            AugmentRarity::Common => 40,
            AugmentRarity::Elite => 70,
            AugmentRarity::Legendary => 120,
        }
    }
}

fn apply_shop_item(
    item: SharedShopItem,
    floor_number: u32,
    effects: &mut PlayerRuleEffects<'_>,
    scaling: &RewardScalingConfig,
) -> bool {
    match item {
        SharedShopItem::Heal => {
            effects.health.current = (effects.health.current + 35.0).min(effects.health.max);
            true
        }
        SharedShopItem::IncreaseMaxHealth => {
            let gain = max_health_gain(scaling, floor_number);
            effects.health.max += gain;
            effects.health.current = (effects.health.current + gain).min(effects.health.max);
            effects.mods.shop_max_health_purchases =
                effects.mods.shop_max_health_purchases.saturating_add(1);
            true
        }
        SharedShopItem::IncreaseAttackPower => {
            effects.attack_power.0 += attack_power_gain(scaling, floor_number);
            effects.mods.shop_attack_power_purchases =
                effects.mods.shop_attack_power_purchases.saturating_add(1);
            true
        }
        SharedShopItem::ReduceDashCooldown => {
            let remain = (0.20 - effects.mods.shop_dash_cooldown_reduction_s).max(0.0);
            if remain <= 0.0 {
                return false;
            }
            effects.mods.shop_dash_cooldown_reduction_s +=
                dash_cooldown_gain_s(scaling, floor_number).min(remain);
            effects
                .dash_cooldown
                .apply_reduction(effects.mods.total_dash_cooldown_reduction());
            effects.mods.shop_dash_purchases = effects.mods.shop_dash_purchases.saturating_add(1);
            true
        }
        SharedShopItem::IncreaseMoveSpeed => {
            let gain = move_speed_gain(scaling, floor_number) * 0.75;
            effects.move_speed.0 += gain;
            effects.mods.shop_move_speed_purchases =
                effects.mods.shop_move_speed_purchases.saturating_add(1);
            true
        }
        SharedShopItem::IncreaseEnergyMax => {
            effects.energy.max += 25.0;
            effects.energy.current = (effects.energy.current + 25.0).min(effects.energy.max);
            true
        }
        SharedShopItem::IncreaseCritChance => {
            let gain = crit_gain(scaling, floor_number) * 0.75;
            let next = (effects.crit.0 + gain).clamp(0.0, 1.0);
            if (next - effects.crit.0).abs() < f32::EPSILON {
                return false;
            }
            effects.crit.0 = next;
            effects.mods.shop_crit_purchases = effects.mods.shop_crit_purchases.saturating_add(1);
            true
        }
        SharedShopItem::IncreaseAttackSpeed => {
            let remain = (0.18 - effects.mods.shop_attack_speed_reduction_s).max(0.0);
            if remain <= 0.0 {
                return false;
            }
            effects.mods.shop_attack_speed_reduction_s +=
                attack_speed_gain_s(scaling, floor_number).min(remain);
            effects
                .attack_cooldown
                .apply_speed_bonus(effects.mods.total_melee_speed_bonus());
            effects
                .ranged_cooldown
                .apply_speed_bonus(effects.mods.total_ranged_speed_bonus());
            effects.mods.shop_attack_speed_purchases =
                effects.mods.shop_attack_speed_purchases.saturating_add(1);
            true
        }
        SharedShopItem::Augment(_) => false,
        SharedShopItem::HealingPotion => {
            let heal = effects.health.max * 0.25;
            effects.health.current = (effects.health.current + heal).min(effects.health.max);
            true
        }
    }
}

fn shop_base_cost(floor_number: u32) -> u32 {
    match floor_number {
        1 => 40,
        2 => 55,
        3 => 70,
        _ => 85,
    }
}

fn shop_item_extra_cost(item: SharedShopItem) -> u32 {
    match item {
        SharedShopItem::Heal => 0,
        SharedShopItem::IncreaseMaxHealth => 15,
        SharedShopItem::IncreaseAttackPower => 18,
        SharedShopItem::ReduceDashCooldown => 18,
        SharedShopItem::IncreaseMoveSpeed => 15,
        SharedShopItem::IncreaseEnergyMax => 12,
        SharedShopItem::IncreaseCritChance => 20,
        SharedShopItem::IncreaseAttackSpeed => 20,
        SharedShopItem::Augment(_) | SharedShopItem::HealingPotion => 0,
    }
}

fn shop_purchase_count(mods: RewardModifiers, item: SharedShopItem) -> u8 {
    match item {
        SharedShopItem::Heal
        | SharedShopItem::IncreaseEnergyMax
        | SharedShopItem::Augment(_)
        | SharedShopItem::HealingPotion => 0,
        SharedShopItem::IncreaseMaxHealth => mods.shop_max_health_purchases,
        SharedShopItem::IncreaseAttackPower => mods.shop_attack_power_purchases,
        SharedShopItem::ReduceDashCooldown => mods.shop_dash_purchases,
        SharedShopItem::IncreaseMoveSpeed => mods.shop_move_speed_purchases,
        SharedShopItem::IncreaseCritChance => mods.shop_crit_purchases,
        SharedShopItem::IncreaseAttackSpeed => mods.shop_attack_speed_purchases,
    }
}

fn shop_repeat_price_mult(purchases: u8) -> f32 {
    match purchases {
        0 => 1.00,
        1 => 1.35,
        2 => 1.75,
        _ => 2.15,
    }
}

fn shop_item_cost(item: SharedShopItem, base_cost: u32, mods: RewardModifiers) -> u32 {
    let base = base_cost + shop_item_extra_cost(item);
    let purchases = shop_purchase_count(mods, item);
    ((base as f32) * shop_repeat_price_mult(purchases)).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_effects() -> (
        Health,
        Energy,
        MoveSpeed,
        AttackPower,
        CritChance,
        DashCooldown,
        AttackCooldown,
        RangedCooldown,
        RewardModifiers,
    ) {
        (
            Health {
                current: 50.0,
                max: 100.0,
            },
            Energy {
                current: 50.0,
                max: 100.0,
            },
            MoveSpeed(280.0),
            AttackPower(20.0),
            CritChance(0.10),
            DashCooldown::new(0.6),
            AttackCooldown::new(0.5),
            RangedCooldown::new(0.8),
            RewardModifiers::default(),
        )
    }

    #[test]
    fn boss_clear_goes_to_victory_on_final_floor() {
        let decision = on_room_cleared(SessionRuleContext {
            mode: SessionMode::Solo,
            floor: 4,
            total_floors: 4,
            boss_gives_victory: false,
            room_type: RoomType::Boss,
        });

        assert_eq!(decision.heal_alive_fraction, 0.80);
        assert_eq!(decision.post_reward, PostRewardDecision::Victory);
    }

    #[test]
    fn boss_clear_goes_to_next_floor_before_final_floor() {
        let decision = on_room_cleared(SessionRuleContext {
            mode: SessionMode::Solo,
            floor: 2,
            total_floors: 4,
            boss_gives_victory: false,
            room_type: RoomType::Boss,
        });

        assert_eq!(decision.post_reward, PostRewardDecision::NextFloor);
    }

    #[test]
    fn shop_refresh_cost_and_repeat_price_follow_curve() {
        assert_eq!(next_refresh_cost(0), 0);
        assert_eq!(next_refresh_cost(1), 50);
        assert_eq!(next_refresh_cost(2), 100);

        let mut mods = RewardModifiers::default();
        let base = shop_item_cost(SharedShopItem::IncreaseAttackPower, shop_base_cost(1), mods);
        mods.shop_attack_power_purchases = 1;
        let raised = shop_item_cost(SharedShopItem::IncreaseAttackPower, shop_base_cost(1), mods);
        assert!(raised > base);
    }

    #[test]
    fn energy_shop_item_increases_energy_max() {
        let scaling = RewardScalingConfig::default_config();
        let (
            mut health,
            mut energy,
            mut move_speed,
            mut attack_power,
            mut crit,
            mut dash,
            mut attack,
            mut ranged,
            mut mods,
        ) = sample_effects();
        let mut effects = PlayerRuleEffects {
            health: &mut health,
            energy: &mut energy,
            move_speed: &mut move_speed,
            attack_power: &mut attack_power,
            crit: &mut crit,
            dash_cooldown: &mut dash,
            attack_cooldown: &mut attack,
            ranged_cooldown: &mut ranged,
            mods: &mut mods,
        };

        let result =
            apply_shop_purchase(SharedShopItem::IncreaseEnergyMax, 1, &mut effects, &scaling);
        assert_eq!(result, ShopPurchaseResult::Applied);
        assert_eq!(effects.energy.max, 125.0);
        assert_eq!(effects.energy.current, 75.0);
    }

    #[test]
    fn death_decision_depends_on_mode_and_living_players() {
        assert_eq!(
            evaluate_death(SessionMode::Solo, 1),
            DeathDecision::Continue
        );
        assert_eq!(
            evaluate_death(SessionMode::Solo, 0),
            DeathDecision::GameOver
        );
        assert_eq!(
            evaluate_death(SessionMode::Coop, 1),
            DeathDecision::Continue
        );
        assert_eq!(
            evaluate_death(SessionMode::Coop, 0),
            DeathDecision::MatchOver
        );
    }
}
