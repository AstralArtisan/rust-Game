use bevy::prelude::*;

use crate::core::achievements::ShopPurchaseEvent;
use crate::core::assets::GameAssets;
use crate::core::input::PlayerInputState;
use crate::data::definitions::RewardScalingConfig;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId, RoomType};
use crate::gameplay::map::transitions::RoomTransition;
use crate::gameplay::player::components::{
    AttackCooldown, AttackPower, CritChance, DashCooldown, Energy, Gold, Health, MoveSpeed, Player,
    RangedCooldown, RewardModifiers,
};
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::session_core::{
    PlayerRuleEffects, SharedShopItem, ShopDraft, ShopOfferDraft, ShopPurchaseResult,
    apply_shop_purchase, build_shop_draft, next_refresh_cost as shared_next_refresh_cost,
    refresh_shop_draft,
};
use crate::states::{AppState, GamePhase};
use crate::ui::feedback::{UiFeedbackEvent, UiFeedbackSeverity};
use crate::ui::skill_select::{
    SkillChoiceOption, SkillChoices, SkillEquipCancelledEvent, SkillEquippedEvent, SkillSelectStep,
};
use crate::utils::rng::GameRng;

pub struct ShopPlugin;
const SHOP_INTERACT_RANGE: f32 = 92.0;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShopOffers>()
            .init_resource::<ShopOfferCache>()
            .init_resource::<ShopSeenRooms>()
            .init_resource::<PendingShopSkillPurchase>()
            .init_resource::<ShopPendingAction>()
            .add_systems(
                Update,
                (
                    reset_shop_state_on_new_floor,
                    spawn_shop_kiosk_if_needed,
                    maybe_enter_shop_state,
                    open_shop_hotkey,
                )
                    .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
            )
            .add_systems(
                Update,
                handle_shop_purchase_input.run_if(in_state(GamePhase::Shop)),
            )
            .add_systems(
                Update,
                handle_shop_skill_equip_result
                    .run_if(in_state(GamePhase::SkillSelect))
                    .after(crate::ui::skill_select::skill_select_input),
            );
    }
}

#[derive(Component)]
pub struct ShopKiosk;

#[derive(Component)]
pub struct ShopUiLine;

#[derive(Resource, Debug, Default, Clone)]
pub struct ShopOffers {
    pub room: Option<RoomId>,
    pub lines: Vec<ShopLine>,
    pub augment_lines: Vec<ShopLine>,
    pub utility_lines: Vec<ShopLine>,
    pub refresh_count: u32,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct ShopOfferCache {
    pub rooms: bevy::utils::HashMap<RoomId, CachedShopState>,
}

#[derive(Resource, Debug, Default)]
pub struct ShopSeenRooms {
    pub rooms: bevy::utils::HashSet<RoomId>,
}

#[derive(Debug, Default, Clone)]
pub struct CachedShopState {
    pub lines: Vec<ShopLine>,
    pub augment_lines: Vec<ShopLine>,
    pub utility_lines: Vec<ShopLine>,
    pub refresh_count: u32,
}

pub fn reset_shop_state_on_new_floor(
    layout: Option<Res<FloorLayout>>,
    mut seen: ResMut<ShopSeenRooms>,
    mut offers: ResMut<ShopOffers>,
    mut cache: ResMut<ShopOfferCache>,
) {
    let Some(layout) = layout else {
        return;
    };
    if !layout.is_changed() {
        return;
    }

    seen.rooms.clear();
    offers.room = None;
    offers.lines.clear();
    offers.augment_lines.clear();
    offers.utility_lines.clear();
    offers.refresh_count = 0;
    cache.rooms.clear();
}

#[derive(Debug, Clone)]
pub struct ShopLine {
    pub title: String,
    pub description: String,
    pub cost: u32,
    pub item: ShopItem,
    pub purchased: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ShopItem {
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
    Skill(crate::gameplay::player::components::SkillType),
    HealingPotion,
    EnergyPotion,
    Talisman,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopSection {
    Attributes,
    Augments,
    Utilities,
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct PendingShopSkillPurchase(pub Option<PendingShopSkillPurchaseInfo>);

#[derive(Debug, Clone, Copy)]
pub struct PendingShopSkillPurchaseInfo {
    pub section: ShopSection,
    pub index: usize,
    pub cost: u32,
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct ShopPendingAction(pub Option<ShopUiAction>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopUiAction {
    Select(ShopSection, usize),
    Refresh,
    Exit,
}

pub fn spawn_shop_kiosk_if_needed(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    existing: Query<(), With<ShopKiosk>>,
) {
    if existing.iter().next().is_some() {
        return;
    }
    let (Some(assets), Some(layout), Some(current)) = (assets, layout, current) else {
        return;
    };
    let Some(room) = layout.room(current.0) else {
        return;
    };
    if room.room_type != RoomType::Shop {
        return;
    }

    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0)),
            sprite: Sprite {
                color: Color::srgb(0.20, 0.70, 0.25),
                custom_size: Some(Vec2::new(60.0, 60.0)),
                ..default()
            },
            ..default()
        },
        ShopKiosk,
        InGameEntity,
        Name::new("ShopKiosk"),
    ));

    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "商店 (E)",
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 22.0,
                    color: Color::WHITE,
                },
            ),
            transform: Transform::from_translation(Vec3::new(0.0, -56.0, 16.0)),
            ..default()
        },
        ShopKiosk,
        InGameEntity,
        Name::new("ShopKioskLabel"),
    ));
}

pub fn maybe_enter_shop_state(
    input: Res<PlayerInputState>,
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    mut seen: ResMut<ShopSeenRooms>,
    mut offers: ResMut<ShopOffers>,
    mut cache: ResMut<ShopOfferCache>,
    mut next: ResMut<NextState<GamePhase>>,
    data: Option<Res<GameDataRegistry>>,
    mut rng: ResMut<GameRng>,
    floor: Option<Res<FloorNumber>>,
    player_q: Query<&GlobalTransform, With<Player>>,
    mods_q: Query<&RewardModifiers, With<Player>>,
    kiosk_q: Query<&GlobalTransform, With<ShopKiosk>>,
    transition: Option<Res<RoomTransition>>,
) {
    if transition
        .as_deref()
        .map(|value| value.active)
        .unwrap_or(false)
    {
        return;
    }
    let (Some(layout), Some(current)) = (layout, current) else {
        return;
    };
    let Some(room) = layout.room(current.0) else {
        return;
    };
    if room.room_type != RoomType::Shop {
        return;
    }

    let auto_open = current.is_changed() && seen.rooms.insert(current.0);
    let manual_open = input.interact_pressed && player_near_shop_kiosk(&player_q, &kiosk_q);
    if !(auto_open || manual_open) {
        return;
    }
    seen.rooms.insert(current.0);

    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
    let mods = mods_q.get_single().copied().unwrap_or_default();
    generate_shop_offers(
        &mut offers,
        &mut cache,
        data.as_deref(),
        &mut rng,
        current.0,
        floor_number,
        mods,
    );
    next.set(GamePhase::Shop);
}

pub fn open_shop_hotkey(
    input: Res<PlayerInputState>,
    mut offers: ResMut<ShopOffers>,
    mut cache: ResMut<ShopOfferCache>,
    mut next: ResMut<NextState<GamePhase>>,
    data: Option<Res<GameDataRegistry>>,
    mut rng: ResMut<GameRng>,
    floor: Option<Res<FloorNumber>>,
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    mods_q: Query<&RewardModifiers, With<Player>>,
    transition: Option<Res<RoomTransition>>,
) {
    if !input.shop_pressed
        || transition
            .as_deref()
            .map(|value| value.active)
            .unwrap_or(false)
    {
        return;
    }
    let (Some(layout), Some(current)) = (layout, current) else {
        return;
    };
    let Some(room) = layout.room(current.0) else {
        return;
    };
    if room.room_type != RoomType::Shop {
        return;
    }

    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
    let mods = mods_q.get_single().copied().unwrap_or_default();
    generate_shop_offers(
        &mut offers,
        &mut cache,
        data.as_deref(),
        &mut rng,
        current.0,
        floor_number,
        mods,
    );
    next.set(GamePhase::Shop);
}

fn generate_shop_offers(
    offers: &mut ShopOffers,
    cache: &mut ShopOfferCache,
    data: Option<&GameDataRegistry>,
    rng: &mut GameRng,
    room: RoomId,
    floor_number: u32,
    mods: RewardModifiers,
) {
    if let Some(state) = cache.rooms.get(&room) {
        offers.room = Some(room);
        offers.lines = state.lines.clone();
        offers.augment_lines = state.augment_lines.clone();
        offers.utility_lines = state.utility_lines.clone();
        offers.refresh_count = state.refresh_count;
        return;
    }

    offers.room = Some(room);
    let draft = build_shop_draft(floor_number, mods, rng, data);
    offers.refresh_count = draft.refresh_count;
    let (lines, augment_lines, utility_lines) = build_shop_lines_from_draft(data, &draft);
    offers.lines = lines;
    offers.augment_lines = augment_lines;
    offers.utility_lines = utility_lines;
    sync_shop_cache(offers, cache, room);
}

fn refresh_shop_offers(
    offers: &mut ShopOffers,
    cache: &mut ShopOfferCache,
    data: Option<&GameDataRegistry>,
    rng: &mut GameRng,
    room: RoomId,
    floor_number: u32,
    mods: RewardModifiers,
) {
    offers.room = Some(room);
    let draft = refresh_shop_draft(offers.refresh_count, floor_number, mods, rng, data);
    offers.refresh_count = draft.refresh_count;
    let (lines, augment_lines, utility_lines) = build_shop_lines_from_draft(data, &draft);
    offers.lines = lines;
    offers.augment_lines = augment_lines;
    offers.utility_lines = utility_lines;
    sync_shop_cache(offers, cache, room);
}

pub fn next_refresh_cost(refresh_count: u32) -> u32 {
    shared_next_refresh_cost(refresh_count)
}

#[allow(dead_code)]
fn describe_item(item: ShopItem, base_cost: u32) -> (&'static str, &'static str, u32) {
    match item {
        ShopItem::Heal => ("治疗药剂", "立刻恢复 30% 最大生命", base_cost),
        ShopItem::RestoreEnergy => ("能量药剂", "立刻恢复 50 能量", base_cost),
        ShopItem::IncreaseMaxHealth => ("强健", "最大生命 +20", base_cost + 10),
        ShopItem::IncreaseAttackPower => ("锋刃", "攻击力 +5", base_cost + 12),
        ShopItem::ReduceDashCooldown => ("迅捷", "冲刺冷却 -15%", base_cost + 12),
        ShopItem::IncreaseMoveSpeed => ("轻盈", "移动速度 +30", base_cost + 10),
        ShopItem::IncreaseEnergyMax => ("充能", "最大能量 +25", base_cost + 8),
        ShopItem::IncreaseCritChance => ("锐眼", "暴击率 +8%", base_cost + 14),
        ShopItem::IncreaseAttackSpeed => ("连击", "攻速 +15%", base_cost + 14),
        ShopItem::Augment(_) => ("强化", "获得一个强化", base_cost),
        ShopItem::UpgradeAugment => ("强化升级", "随机升级一个未满级强化", base_cost),
        ShopItem::Skill(skill) => (skill.label(), "装入一个已解锁终结技槽位", base_cost),
        ShopItem::HealingPotion => ("回血药水", "回复 40% 最大生命", base_cost),
        ShopItem::EnergyPotion => ("能量药水", "回复 60 能量", base_cost),
        ShopItem::Talisman => ("护身符", "下次致命伤保留 1 HP", base_cost),
    }
}

fn describe_item_local(item: ShopItem, base_cost: u32) -> (&'static str, &'static str, u32) {
    match item {
        ShopItem::Heal => ("治疗药剂", "立即恢复生命", base_cost),
        ShopItem::RestoreEnergy => ("能量药剂", "立即恢复能量", base_cost),
        ShopItem::IncreaseMaxHealth => ("强健", "提高生命上限", base_cost + 10),
        ShopItem::IncreaseAttackPower => ("锋刃", "提高攻击伤害", base_cost + 12),
        ShopItem::ReduceDashCooldown => ("迅捷", "缩短冲刺冷却", base_cost + 12),
        ShopItem::IncreaseMoveSpeed => ("轻灵", "提高移动速度", base_cost + 10),
        ShopItem::IncreaseEnergyMax => ("充能", "提高能量上限", base_cost + 8),
        ShopItem::IncreaseCritChance => ("锐眼", "提高暴击率", base_cost + 14),
        ShopItem::IncreaseAttackSpeed => ("连击", "提高攻击节奏", base_cost + 14),
        ShopItem::Augment(_) => ("强化", "获得一个强化", base_cost),
        ShopItem::UpgradeAugment => ("强化升级", "随机升级一个未满级强化", base_cost),
        ShopItem::Skill(skill) => (skill.label(), "装入终结技槽位", base_cost),
        ShopItem::HealingPotion => ("回血药水", "回复 40% 最大生命", base_cost),
        ShopItem::EnergyPotion => ("能量药水", "回复 60 能量", base_cost),
        ShopItem::Talisman => ("护身符", "下次致命伤保留 1 HP", base_cost),
    }
}

fn player_near_shop_kiosk(
    player_q: &Query<&GlobalTransform, With<Player>>,
    kiosk_q: &Query<&GlobalTransform, With<ShopKiosk>>,
) -> bool {
    let Ok(player_tf) = player_q.get_single() else {
        return false;
    };
    let player_pos = player_tf.translation().truncate();
    kiosk_q
        .iter()
        .any(|tf| tf.translation().truncate().distance(player_pos) <= SHOP_INTERACT_RANGE)
}

pub fn handle_shop_purchase_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut pending_action: ResMut<ShopPendingAction>,
    mut offers: ResMut<ShopOffers>,
    mut cache: ResMut<ShopOfferCache>,
    mut pending_skill: ResMut<PendingShopSkillPurchase>,
    mut skill_choices: ResMut<SkillChoices>,
    floor: Option<Res<FloorNumber>>,
    data: Option<Res<GameDataRegistry>>,
    mut rng: ResMut<GameRng>,
    mut next: ResMut<NextState<GamePhase>>,
    mut shop_purchase: EventWriter<ShopPurchaseEvent>,
    mut feedback: EventWriter<UiFeedbackEvent>,
    mut player_q: Query<
        (
            &mut Gold,
            &mut Health,
            &mut Energy,
            &mut MoveSpeed,
            &mut AttackPower,
            &mut CritChance,
            &mut DashCooldown,
            &mut AttackCooldown,
            &mut RangedCooldown,
            &mut RewardModifiers,
            Option<&mut AugmentInventory>,
        ),
        With<Player>,
    >,
) {
    // Esc leaves the shop without buying. Without this the only exit is a
    // successful purchase, trapping a player who has no gold or wants to
    // leave. Offers/cache are preserved so re-entering shows the same state.
    let action = pending_action.0.take();
    if keyboard.just_pressed(KeyCode::Escape) || action == Some(ShopUiAction::Exit) {
        next.set(GamePhase::Playing);
        return;
    }

    let refresh_pressed =
        keyboard.just_pressed(KeyCode::KeyR) || action == Some(ShopUiAction::Refresh);
    let selection = match action {
        Some(ShopUiAction::Select(section, index)) => Some((section, index)),
        _ => shop_selection_from_keyboard(&keyboard),
    };
    let Ok((
        mut gold,
        mut hp,
        mut energy,
        mut move_speed,
        mut power,
        mut crit,
        mut dash_cd,
        mut atk_cd,
        mut ranged_cd,
        mut mods,
        augment_inventory,
    )) = player_q.get_single_mut()
    else {
        return;
    };
    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);

    if refresh_pressed {
        let Some(room) = offers.room else {
            return;
        };
        let cost = next_refresh_cost(offers.refresh_count);
        if gold.0 < cost {
            warn!("金币不足：需要 {}，当前 {}", cost, gold.0);
            feedback.send(shop_warning_feedback(cost, gold.0));
            return;
        }
        gold.0 -= cost;
        refresh_shop_offers(
            &mut offers,
            &mut cache,
            data.as_deref(),
            &mut rng,
            room,
            floor_number,
            *mods,
        );
        feedback.send(UiFeedbackEvent::toast(
            "商店已刷新",
            vec![format!("-{} 金币", cost)],
        ));
        return;
    }

    let Some((section, i)) = selection else {
        return;
    };
    let Some(line) = shop_lines_for_section(&offers, section).get(i).cloned() else {
        return;
    };
    if line.purchased {
        feedback.send(UiFeedbackEvent::toast(
            "已经购买",
            vec![format!("{} 已经售出。", line.title)],
        ));
        return;
    }

    if gold.0 < line.cost {
        warn!("金币不足：需要 {}，当前 {}", line.cost, gold.0);
        feedback.send(shop_warning_feedback(line.cost, gold.0));
        return;
    }

    let mut purchase_lines = vec![format!("-{} 金币", line.cost), line.title.clone()];
    let applied = match line.item {
        ShopItem::Augment(augment_id) => {
            let Some(mut inventory) = augment_inventory else {
                return;
            };
            let grant = inventory.grant(augment_id);
            purchase_lines = vec![format!("-{} 金币", line.cost)];
            purchase_lines.extend(crate::ui::feedback::augment_grant_lines(
                grant,
                data.as_deref(),
            ));
            true
        }
        ShopItem::UpgradeAugment => {
            let Some(mut inventory) = augment_inventory else {
                return;
            };
            let Some(held) = inventory
                .augments
                .iter()
                .find(|held| held.stacks < AugmentInventory::MAX_STACKS)
                .cloned()
            else {
                return;
            };
            let grant = inventory.grant(held.id);
            purchase_lines = vec![format!("-{} 金币", line.cost)];
            purchase_lines.extend(crate::ui::feedback::augment_grant_lines(
                grant,
                data.as_deref(),
            ));
            true
        }
        ShopItem::Skill(skill) => {
            pending_skill.0 = Some(PendingShopSkillPurchaseInfo {
                section,
                index: i,
                cost: line.cost,
            });
            skill_choices.options = vec![SkillChoiceOption {
                skill,
                title: line.title.clone(),
                description: line.description.clone(),
                energy_cost: data
                    .as_deref()
                    .and_then(|registry| registry.skills.get(skill))
                    .map(|config| config.energy_cost)
                    .unwrap_or(0.0),
                cooldown_s: data
                    .as_deref()
                    .and_then(|registry| registry.skills.get(skill))
                    .map(|config| config.cooldown_s)
                    .unwrap_or(0.0),
            }];
            skill_choices.return_state = Some(GamePhase::Playing);
            skill_choices.step = SkillSelectStep::ChooseSkill;
            next.set(GamePhase::SkillSelect);
            return;
        }
        _ => {
            purchase_lines.push(line.description.clone());
            let mut effects = PlayerRuleEffects {
                health: &mut hp,
                energy: &mut energy,
                move_speed: &mut move_speed,
                attack_power: &mut power,
                crit: &mut crit,
                dash_cooldown: &mut dash_cd,
                attack_cooldown: &mut atk_cd,
                ranged_cooldown: &mut ranged_cd,
                mods: &mut mods,
            };
            let scaling = data
                .as_ref()
                .map(|d| &d.rewards.scaling)
                .cloned()
                .unwrap_or_else(RewardScalingConfig::default_config);
            apply_shop_purchase(
                shared_shop_item_from_shop_item(line.item),
                floor_number,
                &mut effects,
                &scaling,
            ) == ShopPurchaseResult::Applied
        }
    };
    if !applied {
        feedback.send(UiFeedbackEvent::toast(
            "购买未完成",
            vec![format!("{} 暂时无法生效。", line.title)],
        ));
        return;
    }
    gold.0 -= line.cost;
    if let Some(slot) = shop_lines_for_section_mut(&mut offers, section).get_mut(i) {
        slot.purchased = true;
    }
    if let Some(room) = offers.room {
        sync_shop_cache(&offers, &mut cache, room);
    }

    shop_purchase.send(ShopPurchaseEvent);
    feedback.send(UiFeedbackEvent::card(
        "购买成功",
        purchase_lines,
        UiFeedbackSeverity::Success,
        GamePhase::Playing,
    ));
    next.set(GamePhase::Playing);
}

fn handle_shop_skill_equip_result(
    mut pending: ResMut<PendingShopSkillPurchase>,
    mut equipped_events: EventReader<SkillEquippedEvent>,
    mut cancelled_events: EventReader<SkillEquipCancelledEvent>,
    mut offers: ResMut<ShopOffers>,
    mut cache: ResMut<ShopOfferCache>,
    mut shop_purchase: EventWriter<ShopPurchaseEvent>,
    mut feedback: EventWriter<UiFeedbackEvent>,
    mut player_q: Query<&mut Gold, With<Player>>,
) {
    if cancelled_events.read().next().is_some() {
        feedback.send(UiFeedbackEvent::toast(
            "终结技购买已取消",
            vec!["未扣除金币，原槽位保持不变。".to_string()],
        ));
        pending.0 = None;
        return;
    }

    let Some(equipped) = equipped_events.read().next() else {
        return;
    };
    let _ = (equipped.skill, equipped.slot);

    let Some(info) = pending.0.take() else {
        return;
    };
    let Ok(mut gold) = player_q.get_single_mut() else {
        return;
    };
    if gold.0 < info.cost {
        warn!(
            "终结技已装入但金币不足以完成商店结算：需要 {}，当前 {}",
            info.cost, gold.0
        );
        feedback.send(shop_warning_feedback(info.cost, gold.0));
        return;
    }

    gold.0 -= info.cost;
    let title = shop_lines_for_section(&offers, info.section)
        .get(info.index)
        .map(|line| line.title.clone())
        .unwrap_or_else(|| equipped.skill.label().to_string());
    if let Some(slot) = shop_lines_for_section_mut(&mut offers, info.section).get_mut(info.index) {
        slot.purchased = true;
    }
    if let Some(room) = offers.room {
        sync_shop_cache(&offers, &mut cache, room);
    }
    shop_purchase.send(ShopPurchaseEvent);
    feedback.send(UiFeedbackEvent::card(
        "终结技购买成功",
        vec![
            format!("-{} 金币", info.cost),
            format!("{} 已装入 {} 槽。", title, equipped.slot.key_label()),
        ],
        UiFeedbackSeverity::Success,
        GamePhase::Playing,
    ));
}

fn shop_warning_feedback(cost: u32, current: u32) -> UiFeedbackEvent {
    UiFeedbackEvent {
        title: "金币不足".to_string(),
        lines: vec![format!("需要 {} 金币，当前只有 {}。", cost, current)],
        severity: UiFeedbackSeverity::Warning,
        requires_ack: false,
        return_phase: GamePhase::Shop,
    }
}

fn build_shop_lines_from_draft(
    data: Option<&GameDataRegistry>,
    draft: &ShopDraft,
) -> (Vec<ShopLine>, Vec<ShopLine>, Vec<ShopLine>) {
    (
        build_shop_section_lines(data, &draft.offers),
        build_shop_section_lines(data, &draft.augment_offers),
        build_shop_section_lines(data, &draft.utility_offers),
    )
}

fn build_shop_section_lines(
    data: Option<&GameDataRegistry>,
    offers: &[ShopOfferDraft],
) -> Vec<ShopLine> {
    offers
        .iter()
        .map(|offer| {
            let item = shop_item_from_shared(offer.item);
            let (title, description) = match item {
                ShopItem::Augment(augment_id) => {
                    augment_details(data, augment_id).unwrap_or_else(|| {
                        let (title, desc, _) = describe_item_local(item, 0);
                        (title.to_string(), desc.to_string())
                    })
                }
                _ => {
                    let (title, desc, _) = describe_item_local(item, 0);
                    (title.to_string(), desc.to_string())
                }
            };
            ShopLine {
                title,
                description,
                cost: offer.cost,
                item,
                purchased: offer.purchased,
            }
        })
        .collect()
}

fn augment_details(
    data: Option<&GameDataRegistry>,
    augment_id: AugmentId,
) -> Option<(String, String)> {
    data.and_then(|registry| {
        registry
            .augments
            .augments
            .iter()
            .find(|augment| augment.id == augment_id)
            .map(|augment| {
                (
                    augment.title.clone(),
                    augment.description_for_stacks(1).to_string(),
                )
            })
    })
}

fn sync_shop_cache(offers: &ShopOffers, cache: &mut ShopOfferCache, room: RoomId) {
    cache.rooms.insert(
        room,
        CachedShopState {
            lines: offers.lines.clone(),
            augment_lines: offers.augment_lines.clone(),
            utility_lines: offers.utility_lines.clone(),
            refresh_count: offers.refresh_count,
        },
    );
}

fn shop_selection_from_keyboard(keyboard: &ButtonInput<KeyCode>) -> Option<(ShopSection, usize)> {
    if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1) {
        Some((ShopSection::Attributes, 0))
    } else if keyboard.just_pressed(KeyCode::Digit2) || keyboard.just_pressed(KeyCode::Numpad2) {
        Some((ShopSection::Attributes, 1))
    } else if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Numpad3) {
        Some((ShopSection::Attributes, 2))
    } else if keyboard.just_pressed(KeyCode::Digit4) || keyboard.just_pressed(KeyCode::Numpad4) {
        Some((ShopSection::Attributes, 3))
    } else if keyboard.just_pressed(KeyCode::Digit5) || keyboard.just_pressed(KeyCode::Numpad5) {
        Some((ShopSection::Augments, 0))
    } else if keyboard.just_pressed(KeyCode::Digit6) || keyboard.just_pressed(KeyCode::Numpad6) {
        Some((ShopSection::Augments, 1))
    } else if keyboard.just_pressed(KeyCode::Digit7) || keyboard.just_pressed(KeyCode::Numpad7) {
        Some((ShopSection::Augments, 2))
    } else if keyboard.just_pressed(KeyCode::Digit8) || keyboard.just_pressed(KeyCode::Numpad8) {
        Some((ShopSection::Augments, 3))
    } else if keyboard.just_pressed(KeyCode::Digit9) || keyboard.just_pressed(KeyCode::Numpad9) {
        Some((ShopSection::Augments, 4))
    } else if keyboard.just_pressed(KeyCode::Digit0) || keyboard.just_pressed(KeyCode::Numpad0) {
        Some((ShopSection::Utilities, 0))
    } else if keyboard.just_pressed(KeyCode::Minus) {
        Some((ShopSection::Utilities, 1))
    } else if keyboard.just_pressed(KeyCode::Equal) {
        Some((ShopSection::Utilities, 2))
    } else {
        None
    }
}

fn shop_lines_for_section(offers: &ShopOffers, section: ShopSection) -> &[ShopLine] {
    match section {
        ShopSection::Attributes => &offers.lines,
        ShopSection::Augments => &offers.augment_lines,
        ShopSection::Utilities => &offers.utility_lines,
    }
}

fn shop_lines_for_section_mut(offers: &mut ShopOffers, section: ShopSection) -> &mut Vec<ShopLine> {
    match section {
        ShopSection::Attributes => &mut offers.lines,
        ShopSection::Augments => &mut offers.augment_lines,
        ShopSection::Utilities => &mut offers.utility_lines,
    }
}

fn shop_item_from_shared(item: SharedShopItem) -> ShopItem {
    match item {
        SharedShopItem::Heal => ShopItem::Heal,
        SharedShopItem::RestoreEnergy => ShopItem::RestoreEnergy,
        SharedShopItem::IncreaseMaxHealth => ShopItem::IncreaseMaxHealth,
        SharedShopItem::IncreaseAttackPower => ShopItem::IncreaseAttackPower,
        SharedShopItem::ReduceDashCooldown => ShopItem::ReduceDashCooldown,
        SharedShopItem::IncreaseMoveSpeed => ShopItem::IncreaseMoveSpeed,
        SharedShopItem::IncreaseEnergyMax => ShopItem::IncreaseEnergyMax,
        SharedShopItem::IncreaseCritChance => ShopItem::IncreaseCritChance,
        SharedShopItem::IncreaseAttackSpeed => ShopItem::IncreaseAttackSpeed,
        SharedShopItem::Augment(augment_id) => ShopItem::Augment(augment_id),
        SharedShopItem::UpgradeAugment => ShopItem::UpgradeAugment,
        SharedShopItem::Skill(skill) => ShopItem::Skill(skill),
        SharedShopItem::HealingPotion => ShopItem::HealingPotion,
        SharedShopItem::EnergyPotion => ShopItem::EnergyPotion,
        SharedShopItem::Talisman => ShopItem::Talisman,
    }
}

fn shared_shop_item_from_shop_item(item: ShopItem) -> SharedShopItem {
    match item {
        ShopItem::Heal => SharedShopItem::Heal,
        ShopItem::RestoreEnergy => SharedShopItem::RestoreEnergy,
        ShopItem::IncreaseMaxHealth => SharedShopItem::IncreaseMaxHealth,
        ShopItem::IncreaseAttackPower => SharedShopItem::IncreaseAttackPower,
        ShopItem::ReduceDashCooldown => SharedShopItem::ReduceDashCooldown,
        ShopItem::IncreaseMoveSpeed => SharedShopItem::IncreaseMoveSpeed,
        ShopItem::IncreaseEnergyMax => SharedShopItem::IncreaseEnergyMax,
        ShopItem::IncreaseCritChance => SharedShopItem::IncreaseCritChance,
        ShopItem::IncreaseAttackSpeed => SharedShopItem::IncreaseAttackSpeed,
        ShopItem::Augment(augment_id) => SharedShopItem::Augment(augment_id),
        ShopItem::UpgradeAugment => SharedShopItem::UpgradeAugment,
        ShopItem::Skill(skill) => SharedShopItem::Skill(skill),
        ShopItem::HealingPotion => SharedShopItem::HealingPotion,
        ShopItem::EnergyPotion => SharedShopItem::EnergyPotion,
        ShopItem::Talisman => SharedShopItem::Talisman,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shop_warning_feedback_keeps_player_in_shop() {
        let feedback = shop_warning_feedback(50, 20);

        assert_eq!(feedback.title, "金币不足");
        assert!(!feedback.requires_ack);
        assert_eq!(feedback.severity, UiFeedbackSeverity::Warning);
        assert_eq!(feedback.return_phase, GamePhase::Shop);
        assert!(feedback.lines.iter().any(|line| line.contains("50")));
    }
}
