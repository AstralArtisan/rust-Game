use crate::data::definitions::RewardScalingConfig;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentRarity};
use crate::gameplay::map::room::RoomType;
use crate::gameplay::player::components::{
    AttackCooldown, AttackPower, CritChance, DashCooldown, Energy, Health, MoveSpeed,
    RangedCooldown, RewardModifiers, SkillType,
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
    RestoreEnergy,
    IncreaseMaxHealth,
    IncreaseAttackPower,
    ReduceDashCooldown,
    IncreaseMoveSpeed,
    IncreaseEnergyMax,
    IncreaseCritChance,
    IncreaseAttackSpeed,
    Augment(AugmentId),
    UpgradeAugment,
    Skill(SkillType),
    HealingPotion,
    EnergyPotion,
    Talisman,
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
    _floor_number: u32,
    mods: RewardModifiers,
    rng: &mut GameRng,
    registry: Option<&GameDataRegistry>,
) -> ShopDraft {
    ShopDraft {
        refresh_count: 0,
        offers: build_shop_offers(mods, registry.map(|registry| &registry.shop)),
        augment_offers: registry
            .map(|registry| build_augment_offers(registry, rng))
            .unwrap_or_default(),
        utility_offers: build_utility_offers(registry.map(|registry| &registry.shop)),
    }
}

pub fn refresh_shop_draft(
    refresh_count: u32,
    _floor_number: u32,
    mods: RewardModifiers,
    rng: &mut GameRng,
    registry: Option<&GameDataRegistry>,
) -> ShopDraft {
    ShopDraft {
        refresh_count: refresh_count.saturating_add(1),
        offers: build_shop_offers(mods, registry.map(|registry| &registry.shop)),
        augment_offers: registry
            .map(|registry| build_augment_offers(registry, rng))
            .unwrap_or_default(),
        utility_offers: build_utility_offers(registry.map(|registry| &registry.shop)),
    }
}

pub fn apply_shop_purchase(
    item: SharedShopItem,
    floor_number: u32,
    effects: &mut PlayerRuleEffects<'_>,
    scaling: &RewardScalingConfig,
    shop_fx: &crate::data::definitions::ShopEffects,
) -> ShopPurchaseResult {
    if apply_shop_item(item, floor_number, effects, scaling, shop_fx) {
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
        30 + refresh_count.saturating_sub(1).saturating_mul(15)
    }
}

fn build_shop_offers(
    mods: RewardModifiers,
    shop: Option<&crate::data::definitions::ShopConfig>,
) -> Vec<ShopOfferDraft> {
    let fallback = crate::data::definitions::ShopConfig::default();
    let shop = shop.unwrap_or(&fallback);
    let inc = &shop.repeat_increment;
    let pool = vec![
        (
            SharedShopItem::Heal,
            shop.heal_price + u32::from(mods.shop_heal_purchases) * inc.heal,
        ),
        (
            SharedShopItem::RestoreEnergy,
            shop.energy_price + u32::from(mods.shop_energy_purchases) * inc.energy,
        ),
        (
            SharedShopItem::IncreaseMaxHealth,
            shop.max_hp_price + u32::from(mods.shop_max_health_purchases) * inc.max_hp,
        ),
        (
            SharedShopItem::IncreaseAttackPower,
            shop.attack_power_price
                + u32::from(mods.shop_attack_power_purchases) * inc.attack_power,
        ),
    ];
    pool.into_iter()
        .map(|item| ShopOfferDraft {
            item: item.0,
            cost: item.1,
            purchased: false,
        })
        .collect()
}

fn build_augment_offers(registry: &GameDataRegistry, rng: &mut GameRng) -> Vec<ShopOfferDraft> {
    let mut pool = registry.augments.augments.iter().collect::<Vec<_>>();
    rng.shuffle(&mut pool);
    pool.truncate(3.min(pool.len()));

    let mut offers = pool
        .into_iter()
        .map(|augment| ShopOfferDraft {
            item: SharedShopItem::Augment(augment.id),
            cost: augment_shop_cost(augment.shop_cost, augment.rarity, &registry.shop),
            purchased: false,
        })
        .collect::<Vec<_>>();
    offers.push(ShopOfferDraft {
        item: SharedShopItem::UpgradeAugment,
        cost: registry.shop.augment_upgrade_price,
        purchased: false,
    });
    let mut skills = registry.skills.skills.iter().collect::<Vec<_>>();
    rng.shuffle(&mut skills);
    for skill in skills.into_iter().take(2) {
        offers.push(ShopOfferDraft {
            item: SharedShopItem::Skill(skill.skill),
            cost: registry.shop.skill_price,
            purchased: false,
        });
    }
    offers
}

fn build_utility_offers(
    shop: Option<&crate::data::definitions::ShopConfig>,
) -> Vec<ShopOfferDraft> {
    let fallback = crate::data::definitions::ShopConfig::default();
    let shop = shop.unwrap_or(&fallback);
    vec![
        ShopOfferDraft {
            item: SharedShopItem::HealingPotion,
            cost: shop.healing_potion_price,
            purchased: false,
        },
        ShopOfferDraft {
            item: SharedShopItem::EnergyPotion,
            cost: shop.energy_potion_price,
            purchased: false,
        },
        ShopOfferDraft {
            item: SharedShopItem::Talisman,
            cost: shop.talisman_price,
            purchased: false,
        },
    ]
}

fn augment_shop_cost(
    shop_cost: u32,
    rarity: AugmentRarity,
    shop: &crate::data::definitions::ShopConfig,
) -> u32 {
    if shop_cost > 0 {
        shop_cost
    } else {
        match rarity {
            AugmentRarity::Common => shop.common_augment_price,
            AugmentRarity::Elite => shop.elite_augment_price,
            AugmentRarity::Legendary => shop.legendary_augment_price,
        }
    }
}

fn apply_shop_item(
    item: SharedShopItem,
    floor_number: u32,
    effects: &mut PlayerRuleEffects<'_>,
    scaling: &RewardScalingConfig,
    shop_fx: &crate::data::definitions::ShopEffects,
) -> bool {
    match item {
        SharedShopItem::Heal => {
            effects.health.current = (effects.health.current
                + effects.health.max * shop_fx.heal_fraction)
                .min(effects.health.max);
            effects.mods.shop_heal_purchases = effects.mods.shop_heal_purchases.saturating_add(1);
            true
        }
        SharedShopItem::RestoreEnergy => {
            effects.energy.current =
                (effects.energy.current + shop_fx.energy_restore).min(effects.energy.max);
            effects.mods.shop_energy_purchases =
                effects.mods.shop_energy_purchases.saturating_add(1);
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
            let remain =
                (shop_fx.dash_cd_cap_s - effects.mods.shop_dash_cooldown_reduction_s).max(0.0);
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
            let gain = move_speed_gain(scaling, floor_number) * shop_fx.move_speed_factor;
            effects.move_speed.0 += gain;
            effects.mods.shop_move_speed_purchases =
                effects.mods.shop_move_speed_purchases.saturating_add(1);
            true
        }
        SharedShopItem::IncreaseEnergyMax => {
            effects.energy.max += shop_fx.energy_max_gain;
            effects.energy.current =
                (effects.energy.current + shop_fx.energy_max_gain).min(effects.energy.max);
            true
        }
        SharedShopItem::IncreaseCritChance => {
            let gain = crit_gain(scaling, floor_number) * shop_fx.crit_factor;
            let next = (effects.crit.0 + gain).clamp(0.0, 1.0);
            if (next - effects.crit.0).abs() < f32::EPSILON {
                return false;
            }
            effects.crit.0 = next;
            effects.mods.shop_crit_purchases = effects.mods.shop_crit_purchases.saturating_add(1);
            true
        }
        SharedShopItem::IncreaseAttackSpeed => {
            let remain =
                (shop_fx.attack_speed_cap_s - effects.mods.shop_attack_speed_reduction_s).max(0.0);
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
        SharedShopItem::Augment(_) | SharedShopItem::UpgradeAugment | SharedShopItem::Skill(_) => {
            false
        }
        SharedShopItem::HealingPotion => {
            let heal = effects.health.max * shop_fx.potion_heal_fraction;
            effects.health.current = (effects.health.current + heal).min(effects.health.max);
            true
        }
        SharedShopItem::EnergyPotion => {
            effects.energy.current =
                (effects.energy.current + shop_fx.energy_potion_restore).min(effects.energy.max);
            true
        }
        SharedShopItem::Talisman => {
            effects.mods.talisman_charges = effects.mods.talisman_charges.saturating_add(1);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::loaders::load_ron;

    fn load_test_registry() -> GameDataRegistry {
        GameDataRegistry {
            player: load_ron("assets/configs/player.ron").unwrap(),
            enemies: load_ron("assets/configs/enemies.ron").unwrap(),
            bosses: load_ron("assets/configs/boss.ron").unwrap(),
            rewards: load_ron("assets/configs/rewards.ron").unwrap(),
            rooms: load_ron("assets/configs/rooms.ron").unwrap(),
            balance: load_ron("assets/configs/game_balance.ron").unwrap(),
            augments: load_ron("assets/configs/augments.ron").unwrap(),
            skills: load_ron("assets/configs/skills.ron").unwrap(),
            events: load_ron("assets/configs/events.ron").unwrap(),
            shop: load_ron("assets/configs/shop.ron").unwrap(),
            economy: load_ron("assets/configs/balance.ron").unwrap(),
            elite_affixes: load_ron("assets/configs/elite_affixes.ron").unwrap(),
            audio: load_ron("assets/configs/audio.ron").unwrap(),
            effects: load_ron("assets/configs/effects.ron").unwrap(),
        }
    }

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
            floor: 3,
            total_floors: 4,
            boss_gives_victory: false,
            room_type: RoomType::Boss,
        });

        assert_eq!(decision.post_reward, PostRewardDecision::NextFloor);
    }

    #[test]
    fn shop_refresh_cost_and_repeat_price_follow_curve() {
        assert_eq!(next_refresh_cost(0), 0);
        assert_eq!(next_refresh_cost(1), 30);
        assert_eq!(next_refresh_cost(2), 45);
        assert_eq!(next_refresh_cost(3), 60);

        let mut mods = RewardModifiers::default();
        let base = build_shop_offers(mods, None)
            .into_iter()
            .find(|offer| offer.item == SharedShopItem::IncreaseAttackPower)
            .map(|offer| offer.cost)
            .unwrap_or_default();
        mods.shop_attack_power_purchases = 1;
        let raised = build_shop_offers(mods, None)
            .into_iter()
            .find(|offer| offer.item == SharedShopItem::IncreaseAttackPower)
            .map(|offer| offer.cost)
            .unwrap_or_default();
        assert!(raised > base);
    }

    #[test]
    fn shop_draft_has_phase3_three_section_structure() {
        let registry = load_test_registry();
        let mut rng = GameRng::default();
        rng.reseed(23);
        let draft = build_shop_draft(1, RewardModifiers::default(), &mut rng, Some(&registry));

        assert_eq!(draft.offers.len(), 4);
        assert!(draft.offers.iter().any(|offer| {
            offer.item == SharedShopItem::Heal && offer.cost == registry.shop.heal_price
        }));
        assert!(draft.offers.iter().any(|offer| {
            offer.item == SharedShopItem::RestoreEnergy && offer.cost == registry.shop.energy_price
        }));
        assert!(draft.offers.iter().any(|offer| {
            offer.item == SharedShopItem::IncreaseMaxHealth
                && offer.cost == registry.shop.max_hp_price
        }));
        assert!(draft.offers.iter().any(|offer| {
            offer.item == SharedShopItem::IncreaseAttackPower
                && offer.cost == registry.shop.attack_power_price
        }));

        assert_eq!(draft.augment_offers.len(), 6);
        assert_eq!(
            draft
                .augment_offers
                .iter()
                .filter(|offer| matches!(offer.item, SharedShopItem::Augment(_)))
                .count(),
            3
        );
        assert!(draft.augment_offers.iter().any(|offer| {
            offer.item == SharedShopItem::UpgradeAugment
                && offer.cost == registry.shop.augment_upgrade_price
        }));
        assert_eq!(
            draft
                .augment_offers
                .iter()
                .filter(|offer| matches!(offer.item, SharedShopItem::Skill(_)))
                .count(),
            2
        );

        assert_eq!(draft.utility_offers.len(), 3);
        assert!(draft.utility_offers.iter().any(|offer| {
            offer.item == SharedShopItem::HealingPotion
                && offer.cost == registry.shop.healing_potion_price
        }));
        assert!(draft.utility_offers.iter().any(|offer| {
            offer.item == SharedShopItem::EnergyPotion
                && offer.cost == registry.shop.energy_potion_price
        }));
        assert!(draft.utility_offers.iter().any(|offer| {
            offer.item == SharedShopItem::Talisman && offer.cost == registry.shop.talisman_price
        }));
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

        let shop_fx = crate::data::definitions::ShopEffects::default();
        let result = apply_shop_purchase(
            SharedShopItem::IncreaseEnergyMax,
            1,
            &mut effects,
            &scaling,
            &shop_fx,
        );
        assert_eq!(result, ShopPurchaseResult::Applied);
        assert_eq!(effects.energy.max, 125.0);
        assert_eq!(effects.energy.current, 75.0);
    }

    #[test]
    fn talisman_shop_item_adds_lethal_guard_charge() {
        let scaling = RewardScalingConfig::default_config();
        let (
            mut health,
            mut energy,
            mut move_speed,
            mut attack_power,
            mut crit,
            mut dash_cooldown,
            mut attack_cooldown,
            mut ranged_cooldown,
            mut mods,
        ) = sample_effects();

        let result = apply_shop_purchase(
            SharedShopItem::Talisman,
            1,
            &mut PlayerRuleEffects {
                health: &mut health,
                energy: &mut energy,
                attack_power: &mut attack_power,
                attack_cooldown: &mut attack_cooldown,
                dash_cooldown: &mut dash_cooldown,
                move_speed: &mut move_speed,
                crit: &mut crit,
                ranged_cooldown: &mut ranged_cooldown,
                mods: &mut mods,
            },
            &scaling,
            &crate::data::definitions::ShopEffects::default(),
        );

        assert_eq!(result, ShopPurchaseResult::Applied);
        assert_eq!(mods.talisman_charges, 1);
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
