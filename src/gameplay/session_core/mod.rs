use crate::data::definitions::{CursesConfig, RunesConfig};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentRarity};
use crate::gameplay::curse::CurseId;
use crate::gameplay::map::room::RoomType;
use crate::gameplay::player::components::{
    AttackCooldown, AttackPower, CritChance, DashCooldown, ENERGY_SYSTEM_ENABLED, Energy, Health,
    MoveSpeed, RangedCooldown, RewardModifiers,
};
use crate::gameplay::rewards::apply::{
    apply_reward_to_player_components, attack_power_gain, attack_speed_gain_s, crit_gain,
    dash_cooldown_gain_s, heal_amount, max_health_gain, move_speed_gain,
};
use crate::gameplay::rewards::data::RewardType;
use crate::gameplay::rune::data::{RuneId, RuneLoadout, RuneSlot, RuneTier};
use crate::utils::rng::GameRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    Solo,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardDraftMode {
    SingleBuff,
    HealOrBuff,
    DualBuff,
    LoneSurvivor,
    Blessing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardOptionDraft {
    Buff(RewardType),
    Rest,
    Revive,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerRuleSnapshot {
    pub player_index: usize,
    pub alive: bool,
    pub mods: RewardModifiers,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRewardDraft {
    pub player_index: usize,
    pub can_interact: bool,
    pub mode: Option<RewardDraftMode>,
    pub primary_options: Vec<RewardOptionDraft>,
    pub secondary_options: Vec<RewardOptionDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardDraft {
    pub lone_survivor: Option<usize>,
    pub players: Vec<PlayerRewardDraft>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RewardSelection {
    pub mode: RewardDraftMode,
    pub primary: Option<RewardOptionDraft>,
    pub secondary: Option<RewardOptionDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlessingOffer {
    pub rune_id: RuneId,
    pub rune_slot: RuneSlot,
    pub rune_tier: RuneTier,
    pub rune_title: String,
    pub rune_description: String,
    pub rune_drawback: String,
    pub curse_id: CurseId,
    pub curse_title: String,
    pub curse_description: String,
    pub curse_duration: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostRewardDecision {
    ResumeRun,
    NextFloor,
    Victory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomEnterDecision {
    pub reward_mode: Option<RewardDraftMode>,
    pub auto_open_shop: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomClearDecision {
    pub reward_mode: Option<RewardDraftMode>,
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
    RemoveCurse,
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

pub fn on_room_enter(
    ctx: SessionRuleContext,
    first_visit: bool,
    has_active_curse: bool,
) -> RoomEnterDecision {
    if !first_visit {
        return RoomEnterDecision {
            reward_mode: None,
            auto_open_shop: false,
        };
    }

    match ctx.room_type {
        RoomType::Reward => {
            if ctx.floor <= 1 || has_active_curse {
                RoomEnterDecision {
                    reward_mode: None,
                    auto_open_shop: false,
                }
            } else {
                RoomEnterDecision {
                    reward_mode: Some(RewardDraftMode::Blessing),
                    auto_open_shop: false,
                }
            }
        }
        RoomType::Shop => RoomEnterDecision {
            reward_mode: None,
            auto_open_shop: true,
        },
        _ => RoomEnterDecision {
            reward_mode: None,
            auto_open_shop: false,
        },
    }
}

pub fn on_room_cleared(ctx: SessionRuleContext) -> RoomClearDecision {
    match ctx.room_type {
        RoomType::Normal => RoomClearDecision {
            reward_mode: Some(RewardDraftMode::HealOrBuff),
            heal_alive_fraction: 0.0,
            post_reward: PostRewardDecision::ResumeRun,
        },
        RoomType::Boss => {
            let reached_final_floor = ctx.floor >= ctx.total_floors.max(1);
            let post_reward = if ctx.boss_gives_victory || reached_final_floor {
                PostRewardDecision::Victory
            } else {
                PostRewardDecision::NextFloor
            };
            RoomClearDecision {
                reward_mode: Some(RewardDraftMode::DualBuff),
                heal_alive_fraction: 0.80,
                post_reward,
            }
        }
        RoomType::Event => RoomClearDecision {
            reward_mode: None,
            heal_alive_fraction: 0.0,
            post_reward: PostRewardDecision::ResumeRun,
        },
        _ => RoomClearDecision {
            reward_mode: None,
            heal_alive_fraction: 0.0,
            post_reward: PostRewardDecision::ResumeRun,
        },
    }
}

pub fn build_reward_draft(
    session_mode: SessionMode,
    mode: RewardDraftMode,
    rng: &mut GameRng,
    players: &[PlayerRuleSnapshot],
) -> RewardDraft {
    let mut draft = RewardDraft {
        lone_survivor: None,
        players: players
            .iter()
            .map(|player| PlayerRewardDraft {
                player_index: player.player_index,
                can_interact: false,
                mode: None,
                primary_options: Vec::new(),
                secondary_options: Vec::new(),
            })
            .collect(),
    };

    let living = players
        .iter()
        .filter(|player| player.alive)
        .copied()
        .collect::<Vec<_>>();

    if session_mode == SessionMode::Coop && living.len() == 1 {
        let survivor = living[0];
        draft.lone_survivor = Some(survivor.player_index);
        if let Some(player) = draft
            .players
            .iter_mut()
            .find(|player| player.player_index == survivor.player_index)
        {
            let buff = generate_reward_choices(rng, survivor.mods, &[])
                .into_iter()
                .next()
                .unwrap_or(RewardType::IncreaseAttackPower);
            player.can_interact = true;
            player.mode = Some(RewardDraftMode::LoneSurvivor);
            player.primary_options = vec![
                RewardOptionDraft::Rest,
                RewardOptionDraft::Revive,
                RewardOptionDraft::Buff(buff),
            ];
        }
        return draft;
    }

    for snapshot in players {
        let player = draft
            .players
            .iter_mut()
            .find(|player| player.player_index == snapshot.player_index);
        let Some(player) = player else {
            continue;
        };

        if !snapshot.alive {
            continue;
        }

        let (primary_options, secondary_options) =
            reward_options_for_mode(mode, rng, snapshot.mods);
        player.can_interact = true;
        player.mode = Some(mode);
        player.primary_options = primary_options;
        player.secondary_options = secondary_options;
    }

    draft
}

pub fn apply_reward_selection(
    selection: RewardSelection,
    floor_number: u32,
    effects: &mut PlayerRuleEffects<'_>,
    post_reward: PostRewardDecision,
) -> PostRewardDecision {
    for option in [selection.primary, selection.secondary]
        .into_iter()
        .flatten()
    {
        let _ = apply_reward_option(option, selection.mode, floor_number, effects);
    }
    post_reward
}

pub fn reward_selection_requests_revive(selection: RewardSelection) -> bool {
    [selection.primary, selection.secondary]
        .into_iter()
        .flatten()
        .any(reward_option_requests_revive)
}

pub fn reward_option_requests_revive(option: RewardOptionDraft) -> bool {
    option == RewardOptionDraft::Revive
}

pub fn build_shop_draft(
    floor_number: u32,
    mods: RewardModifiers,
    rng: &mut GameRng,
    registry: Option<&GameDataRegistry>,
    has_curse: bool,
) -> ShopDraft {
    ShopDraft {
        refresh_count: 0,
        offers: build_shop_offers(floor_number, mods, rng),
        augment_offers: registry
            .map(|registry| build_augment_offers(registry, rng, floor_number))
            .unwrap_or_default(),
        utility_offers: build_utility_offers(has_curse),
    }
}

pub fn refresh_shop_draft(
    refresh_count: u32,
    floor_number: u32,
    mods: RewardModifiers,
    rng: &mut GameRng,
    registry: Option<&GameDataRegistry>,
    has_curse: bool,
) -> ShopDraft {
    ShopDraft {
        refresh_count: refresh_count.saturating_add(1),
        offers: build_shop_offers(floor_number, mods, rng),
        augment_offers: registry
            .map(|registry| build_augment_offers(registry, rng, floor_number))
            .unwrap_or_default(),
        utility_offers: build_utility_offers(has_curse),
    }
}

pub fn apply_shop_purchase(
    item: SharedShopItem,
    floor_number: u32,
    effects: &mut PlayerRuleEffects<'_>,
) -> ShopPurchaseResult {
    if apply_shop_item(item, floor_number, effects) {
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

pub fn can_advance_room(
    mode: SessionMode,
    room_type: RoomType,
    room_cleared: bool,
    reward_resolved: bool,
) -> bool {
    !room_cleared || !room_clear_requires_reward(mode, room_type) || reward_resolved
}

pub fn next_refresh_cost(refresh_count: u32) -> u32 {
    if refresh_count == 0 {
        0
    } else {
        refresh_count.saturating_mul(50)
    }
}

fn room_clear_requires_reward(mode: SessionMode, room_type: RoomType) -> bool {
    match mode {
        SessionMode::Solo => matches!(
            room_type,
            RoomType::Normal | RoomType::Boss
        ),
        SessionMode::Coop => matches!(room_type, RoomType::Normal | RoomType::Boss),
    }
}

fn reward_options_for_mode(
    mode: RewardDraftMode,
    rng: &mut GameRng,
    mods: RewardModifiers,
) -> (Vec<RewardOptionDraft>, Vec<RewardOptionDraft>) {
    match mode {
        RewardDraftMode::SingleBuff => (
            generate_reward_choices(rng, mods, &[])
                .into_iter()
                .map(RewardOptionDraft::Buff)
                .collect(),
            Vec::new(),
        ),
        RewardDraftMode::HealOrBuff => {
            let mut primary = vec![RewardOptionDraft::Rest];
            primary.extend(
                generate_reward_choices(rng, mods, &[])
                    .into_iter()
                    .map(RewardOptionDraft::Buff),
            );
            (primary, Vec::new())
        }
        RewardDraftMode::DualBuff => {
            let (primary, secondary) = generate_dual_reward_choices(rng, mods);
            (
                primary.into_iter().map(RewardOptionDraft::Buff).collect(),
                secondary.into_iter().map(RewardOptionDraft::Buff).collect(),
            )
        }
        RewardDraftMode::LoneSurvivor => {
            let buff = generate_reward_choices(rng, mods, &[])
                .into_iter()
                .next()
                .unwrap_or(RewardType::IncreaseAttackPower);
            (
                vec![
                    RewardOptionDraft::Rest,
                    RewardOptionDraft::Revive,
                    RewardOptionDraft::Buff(buff),
                ],
                Vec::new(),
            )
        }
        RewardDraftMode::Blessing => (Vec::new(), Vec::new()),
    }
}

fn apply_reward_option(
    option: RewardOptionDraft,
    mode: RewardDraftMode,
    floor_number: u32,
    effects: &mut PlayerRuleEffects<'_>,
) -> bool {
    match option {
        RewardOptionDraft::Buff(reward) => {
            apply_reward_to_player_components(
                reward,
                floor_number,
                reward_scale_for_mode(mode),
                effects.mods,
                effects.health,
                effects.move_speed,
                effects.dash_cooldown,
                effects.ranged_cooldown,
                effects.crit,
                effects.attack_cooldown,
                effects.attack_power,
            );
            false
        }
        RewardOptionDraft::Rest => {
            let heal = heal_amount(effects.health.max, floor_number);
            effects.health.current = (effects.health.current + heal).min(effects.health.max);
            false
        }
        RewardOptionDraft::Revive => true,
    }
}

fn reward_scale_for_mode(mode: RewardDraftMode) -> f32 {
    match mode {
        RewardDraftMode::DualBuff => 1.50,
        _ => 1.0,
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

fn build_utility_offers(has_curse: bool) -> Vec<ShopOfferDraft> {
    let mut offers = vec![ShopOfferDraft {
        item: SharedShopItem::HealingPotion,
        cost: 30,
        purchased: false,
    }];
    if has_curse {
        offers.push(ShopOfferDraft {
            item: SharedShopItem::RemoveCurse,
            cost: 80,
            purchased: false,
        });
    }
    offers
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
) -> bool {
    match item {
        SharedShopItem::Heal => {
            effects.health.current = (effects.health.current + 35.0).min(effects.health.max);
            true
        }
        SharedShopItem::IncreaseMaxHealth => {
            let gain = max_health_gain(floor_number);
            effects.health.max += gain;
            effects.health.current = (effects.health.current + gain).min(effects.health.max);
            effects.mods.shop_max_health_purchases =
                effects.mods.shop_max_health_purchases.saturating_add(1);
            true
        }
        SharedShopItem::IncreaseAttackPower => {
            effects.attack_power.0 += attack_power_gain(floor_number);
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
                dash_cooldown_gain_s(floor_number).min(remain);
            effects
                .dash_cooldown
                .apply_reduction(effects.mods.total_dash_cooldown_reduction());
            effects.mods.shop_dash_purchases = effects.mods.shop_dash_purchases.saturating_add(1);
            true
        }
        SharedShopItem::IncreaseMoveSpeed => {
            let gain = move_speed_gain(floor_number) * 0.75;
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
            let gain = crit_gain(floor_number) * 0.75;
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
                attack_speed_gain_s(floor_number).min(remain);
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
        SharedShopItem::RemoveCurse => false,
    }
}

fn generate_reward_choices(
    rng: &mut GameRng,
    mods: RewardModifiers,
    excluded: &[RewardType],
) -> Vec<RewardType> {
    let mut pool = reward_pool()
        .into_iter()
        .filter(|reward| !mods.reward_at_max(*reward) && !excluded.contains(reward))
        .collect::<Vec<_>>();
    if pool.len() < 3 {
        pool = reward_pool()
            .into_iter()
            .filter(|reward| !excluded.contains(reward))
            .collect::<Vec<_>>();
    }
    rng.shuffle(&mut pool);
    pool.truncate(3);
    pool
}

fn generate_dual_reward_choices(
    rng: &mut GameRng,
    mods: RewardModifiers,
) -> (Vec<RewardType>, Vec<RewardType>) {
    let primary = generate_reward_choices(rng, mods, &[]);
    let mut secondary = generate_reward_choices(rng, mods, &primary);
    if secondary.len() < 3 {
        secondary = generate_reward_choices(rng, mods, &[]);
    }
    (primary, secondary)
}

pub fn generate_blessing_choices(
    rng: &mut GameRng,
    floor_number: u32,
    rune_loadout: &RuneLoadout,
    runes_config: &RunesConfig,
    curses_config: &CursesConfig,
) -> Vec<BlessingOffer> {
    if curses_config.curses.is_empty() {
        return Vec::new();
    }

    let equipped = RuneSlot::ALL
        .into_iter()
        .filter_map(|slot| rune_loadout.get(slot))
        .collect::<Vec<_>>();
    let mut available = runes_config
        .runes
        .iter()
        .filter(|rune| !equipped.contains(&rune.id))
        .collect::<Vec<_>>();
    if available.is_empty() {
        return Vec::new();
    }

    let mut selected = Vec::new();
    if floor_number >= 4 {
        let mut legendaries = available
            .iter()
            .copied()
            .filter(|rune| rune.tier == RuneTier::Legendary)
            .collect::<Vec<_>>();
        rng.shuffle(&mut legendaries);
        if let Some(legendary) = legendaries.into_iter().next() {
            selected.push(legendary);
            available.retain(|rune| rune.id != legendary.id);
        }
    }

    let preferred_tiers = if floor_number >= 4 {
        [RuneTier::Elite, RuneTier::Common, RuneTier::Legendary]
    } else {
        [RuneTier::Elite, RuneTier::Common, RuneTier::Legendary]
    };
    let mut ordered = ordered_rune_candidates(rng, &available, &preferred_tiers);
    while selected.len() < 2 {
        let Some(next) = ordered.pop() else {
            break;
        };
        if selected.iter().any(|rune| rune.id == next.id) {
            continue;
        }
        if floor_number < 4 && next.tier == RuneTier::Legendary {
            continue;
        }
        selected.push(next);
    }

    if selected.len() < 2 {
        let mut fallback = available;
        rng.shuffle(&mut fallback);
        for rune in fallback {
            if selected.iter().any(|picked| picked.id == rune.id) {
                continue;
            }
            selected.push(rune);
            if selected.len() == 2 {
                break;
            }
        }
    }

    let mut curses = curses_config.curses.iter().collect::<Vec<_>>();
    rng.shuffle(&mut curses);

    selected
        .into_iter()
        .take(2)
        .enumerate()
        .filter_map(|(index, rune)| {
            let curse = curses
                .get(index)
                .copied()
                .or_else(|| curses.first().copied())?;
            Some(BlessingOffer {
                rune_id: rune.id,
                rune_slot: rune.slot,
                rune_tier: rune.tier,
                rune_title: rune.title.clone(),
                rune_description: rune.description.clone(),
                rune_drawback: rune.drawback.clone(),
                curse_id: curse.id,
                curse_title: curse.title.clone(),
                curse_description: curse.description.clone(),
                curse_duration: curse.duration,
            })
        })
        .collect()
}

fn ordered_rune_candidates<'a>(
    rng: &mut GameRng,
    available: &[&'a crate::data::definitions::RuneConfig],
    preferred_tiers: &[RuneTier],
) -> Vec<&'a crate::data::definitions::RuneConfig> {
    let mut ordered = Vec::new();
    for tier in preferred_tiers {
        let mut tier_pool = available
            .iter()
            .copied()
            .filter(|rune| rune.tier == *tier)
            .collect::<Vec<_>>();
        rng.shuffle(&mut tier_pool);
        ordered.extend(tier_pool);
    }
    ordered.reverse();
    ordered
}

fn reward_pool() -> Vec<RewardType> {
    vec![
        RewardType::EnhanceMeleeWeapon,
        RewardType::IncreaseAttackSpeed,
        RewardType::IncreaseAttackPower,
        RewardType::IncreaseMaxHealth,
        RewardType::ReduceDashCooldown,
        RewardType::LifeStealOnKill,
        RewardType::IncreaseCritChance,
        RewardType::IncreaseMoveSpeed,
        RewardType::DashDamageTrail,
        RewardType::EnhanceRangedWeapon,
    ]
}

fn shop_base_cost(floor_number: u32) -> u32 {
    match floor_number {
        1 => 55,
        2 => 65,
        3 => 75,
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
        SharedShopItem::Augment(_)
        | SharedShopItem::HealingPotion
        | SharedShopItem::RemoveCurse => 0,
    }
}

fn shop_purchase_count(mods: RewardModifiers, item: SharedShopItem) -> u8 {
    match item {
        SharedShopItem::Heal
        | SharedShopItem::IncreaseEnergyMax
        | SharedShopItem::Augment(_)
        | SharedShopItem::HealingPotion
        | SharedShopItem::RemoveCurse => 0,
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

    fn seeded_rng(seed: u64) -> GameRng {
        let mut rng = GameRng::default();
        rng.reseed(seed);
        rng
    }

    fn snapshot(index: usize, alive: bool) -> PlayerRuleSnapshot {
        PlayerRuleSnapshot {
            player_index: index,
            alive,
            mods: RewardModifiers::default(),
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
    fn reward_room_generates_single_buff_for_each_living_player() {
        let mut rng = seeded_rng(1);
        let draft = build_reward_draft(
            SessionMode::Coop,
            RewardDraftMode::SingleBuff,
            &mut rng,
            &[snapshot(0, true), snapshot(1, true)],
        );

        assert_eq!(draft.lone_survivor, None);
        for player in draft.players {
            assert!(player.can_interact);
            assert_eq!(player.mode, Some(RewardDraftMode::SingleBuff));
            assert_eq!(player.primary_options.len(), 3);
            assert!(player.secondary_options.is_empty());
        }
    }

    #[test]
    fn normal_clear_generates_heal_plus_three_buffs() {
        let mut rng = seeded_rng(2);
        let draft = build_reward_draft(
            SessionMode::Coop,
            RewardDraftMode::HealOrBuff,
            &mut rng,
            &[snapshot(0, true), snapshot(1, true)],
        );

        for player in draft.players {
            assert!(player.can_interact);
            assert_eq!(player.mode, Some(RewardDraftMode::HealOrBuff));
            assert_eq!(player.primary_options.len(), 4);
            assert_eq!(player.primary_options[0], RewardOptionDraft::Rest);
            assert!(player.secondary_options.is_empty());
        }
    }

    #[test]
    fn boss_clear_generates_dual_reward_columns() {
        let mut rng = seeded_rng(3);
        let draft = build_reward_draft(
            SessionMode::Coop,
            RewardDraftMode::DualBuff,
            &mut rng,
            &[snapshot(0, true), snapshot(1, true)],
        );

        for player in draft.players {
            assert!(player.can_interact);
            assert_eq!(player.mode, Some(RewardDraftMode::DualBuff));
            assert_eq!(player.primary_options.len(), 3);
            assert_eq!(player.secondary_options.len(), 3);
        }
    }

    #[test]
    fn coop_lone_survivor_generates_rest_revive_buff() {
        let mut rng = seeded_rng(4);
        let draft = build_reward_draft(
            SessionMode::Coop,
            RewardDraftMode::HealOrBuff,
            &mut rng,
            &[snapshot(0, true), snapshot(1, false)],
        );

        assert_eq!(draft.lone_survivor, Some(0));
        assert_eq!(
            draft.players[0].primary_options,
            vec![
                RewardOptionDraft::Rest,
                RewardOptionDraft::Revive,
                draft.players[0].primary_options[2],
            ]
        );
        assert_eq!(draft.players[0].mode, Some(RewardDraftMode::LoneSurvivor));
        assert!(!draft.players[1].can_interact);
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

        assert_eq!(decision.reward_mode, Some(RewardDraftMode::DualBuff));
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

        let result = apply_shop_purchase(SharedShopItem::IncreaseEnergyMax, 1, &mut effects);
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

    #[test]
    fn cannot_advance_room_until_reward_is_resolved() {
        assert!(!can_advance_room(
            SessionMode::Coop,
            RoomType::Normal,
            true,
            false,
        ));
        assert!(can_advance_room(
            SessionMode::Coop,
            RoomType::Normal,
            true,
            true,
        ));
        assert!(can_advance_room(
            SessionMode::Coop,
            RoomType::Shop,
            true,
            false,
        ));
    }

    #[test]
    fn apply_reward_selection_returns_post_reward_decision() {
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

        let post = apply_reward_selection(
            RewardSelection {
                mode: RewardDraftMode::HealOrBuff,
                primary: Some(RewardOptionDraft::Rest),
                secondary: None,
            },
            1,
            &mut effects,
            PostRewardDecision::ResumeRun,
        );

        assert_eq!(post, PostRewardDecision::ResumeRun);
        assert!(effects.health.current > 50.0);
    }
}
