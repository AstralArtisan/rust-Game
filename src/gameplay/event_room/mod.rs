use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::core::events::RoomClearedEvent;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory, AugmentRarity};
use crate::gameplay::curse::{CurseId, CurseState};
use crate::gameplay::enemy::components::EnemyType;
use crate::gameplay::enemy::spawner;
use crate::gameplay::enemy::systems::{spawn_enemy, spawn_room_enemies};
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId, RoomType};
use crate::gameplay::player::components::{Gold, Health, Player};
use crate::gameplay::progression::difficulty::{
    get_floor_difficulty_multiplier, get_floor_enemy_count,
};
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::puzzle;
use crate::states::{AppState, RoomState};
use crate::utils::rng::GameRng;

pub struct EventRoomPlugin;

impl Plugin for EventRoomPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveEvent>()
            .add_systems(
                Update,
                select_and_spawn_event
                    .after(crate::gameplay::enemy::systems::room_entry_spawner)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                tick_timed_challenge.run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                event_room_input.run_if(in_state(AppState::EventRoom)),
            )
            .add_systems(Update, resolve_event_room_clear);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    PressurePlate,
    SwitchOrder,
    TrapSurvival,
    Gambler,
    CurseAltar,
    BloodPact,
    Treasure,
    HealingSpring,
    Merchant,
    TimedChallenge,
    EliteEncounter,
}

impl EventType {
    pub fn title(self) -> &'static str {
        match self {
            Self::PressurePlate => "踏板试炼",
            Self::SwitchOrder => "机关顺序",
            Self::TrapSurvival => "陷阱求生",
            Self::Gambler => "赌徒",
            Self::CurseAltar => "诅咒祭坛",
            Self::BloodPact => "血契",
            Self::Treasure => "宝箱",
            Self::HealingSpring => "治愈泉",
            Self::Merchant => "流动商贩",
            Self::TimedChallenge => "限时清剿",
            Self::EliteEncounter => "精英决斗",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::PressurePlate => "完成踏板机关即可通关。",
            Self::SwitchOrder => "按正确顺序触发机关。",
            Self::TrapSurvival => "在陷阱区域中撑过挑战。",
            Self::Gambler => "付出金币，赌一项随机强化。",
            Self::CurseAltar => "接受诅咒，换取更强的力量。",
            Self::BloodPact => "献出生命，换取一项强化。",
            Self::Treasure => "挑选宝藏并顺手带走金币。",
            Self::HealingSpring => "停留片刻，恢复生命。",
            Self::Merchant => "挑一件半价强化带走。",
            Self::TimedChallenge => "30 秒内清空房间，可得精英强化。",
            Self::EliteEncounter => "击败单个精英，直接获得精英强化。",
        }
    }

    pub fn is_puzzle(self) -> bool {
        matches!(
            self,
            Self::PressurePlate | Self::SwitchOrder | Self::TrapSurvival
        )
    }

    fn is_combat(self) -> bool {
        matches!(self, Self::TimedChallenge | Self::EliteEncounter)
    }
}

#[derive(Debug, Clone)]
pub struct EventChoice {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone)]
enum EventChoicePayload {
    Leave,
    Gambler { cost: u32, augment_id: AugmentId },
    CurseAltar {
        curse_id: CurseId,
        duration: u32,
        augment_id: AugmentId,
    },
    BloodPact { augment_id: AugmentId },
    Treasure { augment_id: AugmentId, gold_bonus: u32 },
    HealingSpring,
    Merchant { cost: u32, augment_id: AugmentId },
}

#[derive(Resource, Debug, Clone)]
pub struct ActiveEvent {
    pub event_type: Option<EventType>,
    pub resolved: bool,
    pub room: Option<RoomId>,
    pub choices: Vec<EventChoice>,
    choice_payloads: Vec<EventChoicePayload>,
    combat_reward_ready: bool,
    timed_challenge_timer: Option<Timer>,
}

impl Default for ActiveEvent {
    fn default() -> Self {
        Self {
            event_type: None,
            resolved: false,
            room: None,
            choices: Vec::new(),
            choice_payloads: Vec::new(),
            combat_reward_ready: false,
            timed_challenge_timer: None,
        }
    }
}

pub fn reset_active_event(active: &mut ActiveEvent) {
    *active = ActiveEvent::default();
}

fn mark_event_resolved(active: &mut ActiveEvent) {
    active.resolved = true;
    active.choices.clear();
    active.choice_payloads.clear();
    active.combat_reward_ready = false;
    active.timed_challenge_timer = None;
}

fn select_and_spawn_event(
    mut commands: Commands,
    assets: Res<GameAssets>,
    data: Option<Res<GameDataRegistry>>,
    layout: Option<Res<FloorLayout>>,
    current_room: Option<Res<CurrentRoom>>,
    mut room_state: ResMut<RoomState>,
    mut active: ResMut<ActiveEvent>,
    mut active_puzzle: ResMut<puzzle::ActivePuzzle>,
    mut rng: ResMut<GameRng>,
    mut next_state: ResMut<NextState<AppState>>,
    floor: Option<Res<FloorNumber>>,
) {
    let (Some(layout), Some(current_room)) = (layout, current_room) else {
        return;
    };
    let Some(room) = layout.room(current_room.0) else {
        return;
    };
    if room.room_type != RoomType::Event {
        return;
    }
    if active.room == Some(current_room.0) && (active.resolved || active.event_type.is_some()) {
        return;
    }
    if active.room != Some(current_room.0) {
        reset_active_event(&mut active);
        active.room = Some(current_room.0);
    }

    let event_type = pick_weighted_event(&mut rng);
    active.event_type = Some(event_type);
    active.resolved = false;
    active.choices.clear();
    active.choice_payloads.clear();
    active.combat_reward_ready = event_type.is_combat();
    active.timed_challenge_timer = None;

    match event_type {
        EventType::PressurePlate | EventType::SwitchOrder | EventType::TrapSurvival => {
            *room_state = RoomState::Locked;
            puzzle::spawn_puzzle_for_room(
                &mut commands,
                &assets,
                &mut rng,
                &mut active_puzzle,
                current_room.0,
            );
        }
        EventType::TimedChallenge => {
            let Some(data) = data.as_deref() else {
                mark_event_resolved(&mut active);
                return;
            };
            *room_state = RoomState::Locked;
            active.combat_reward_ready = true;
            active.timed_challenge_timer =
                Some(Timer::from_seconds(30.0, TimerMode::Once));
            let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
            let enemy_count = get_floor_enemy_count(data, floor_number).max(4);
            let floor_multiplier = get_floor_difficulty_multiplier(data, floor_number);
            spawn_room_enemies(
                &mut commands,
                &assets,
                data,
                enemy_count,
                floor_multiplier,
                floor_number,
                1.0,
            );
        }
        EventType::EliteEncounter => {
            let Some(data) = data.as_deref() else {
                mark_event_resolved(&mut active);
                return;
            };
            *room_state = RoomState::Locked;
            active.combat_reward_ready = true;
            let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
            let floor_multiplier = get_floor_difficulty_multiplier(data, floor_number);
            spawn_elite_event_enemy(
                &mut commands,
                &assets,
                data,
                &mut rng,
                floor_number,
                floor_multiplier,
            );
        }
        EventType::Gambler
        | EventType::CurseAltar
        | EventType::BloodPact
        | EventType::Treasure
        | EventType::HealingSpring
        | EventType::Merchant => {
            configure_non_combat_event(&mut active, data.as_deref(), &mut rng);
            next_state.set(AppState::EventRoom);
        }
    }
}

fn tick_timed_challenge(time: Res<Time>, mut active: ResMut<ActiveEvent>) {
    if active.event_type != Some(EventType::TimedChallenge) || active.resolved {
        return;
    }
    let Some(timer) = active.timed_challenge_timer.as_mut() else {
        return;
    };
    timer.tick(time.delta());
    if timer.finished() {
        active.combat_reward_ready = false;
        active.timed_challenge_timer = None;
    }
}

fn event_room_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut active: ResMut<ActiveEvent>,
    mut room_state: ResMut<RoomState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut cleared: EventWriter<RoomClearedEvent>,
    mut player_q: Query<(&mut Gold, &mut Health, &mut AugmentInventory, &mut CurseState), With<Player>>,
) {
    let Some(room) = active.room else {
        next_state.set(AppState::InGame);
        return;
    };

    if keyboard.just_pressed(KeyCode::Escape) {
        mark_event_resolved(&mut active);
        next_state.set(AppState::InGame);
        return;
    }

    let index = if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1)
    {
        Some(0)
    } else if keyboard.just_pressed(KeyCode::Digit2) || keyboard.just_pressed(KeyCode::Numpad2) {
        Some(1)
    } else if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Numpad3) {
        Some(2)
    } else {
        None
    };
    let Some(index) = index else {
        return;
    };
    let Some(payload) = active.choice_payloads.get(index).cloned() else {
        return;
    };
    let Ok((mut gold, mut health, mut inventory, mut curses)) = player_q.get_single_mut() else {
        return;
    };

    match apply_choice_payload(payload, &mut gold, &mut health, &mut inventory, &mut curses) {
        EventInputOutcome::StayOpen => {}
        EventInputOutcome::Leave => {
            mark_event_resolved(&mut active);
            next_state.set(AppState::InGame);
        }
        EventInputOutcome::Complete => {
            *room_state = RoomState::Cleared;
            mark_event_resolved(&mut active);
            cleared.send(RoomClearedEvent { room });
            next_state.set(AppState::InGame);
        }
    }
}

fn resolve_event_room_clear(
    mut cleared: EventReader<RoomClearedEvent>,
    mut active: ResMut<ActiveEvent>,
    data: Option<Res<GameDataRegistry>>,
    mut rng: ResMut<GameRng>,
    mut player_q: Query<&mut AugmentInventory, With<Player>>,
) {
    for ev in cleared.read() {
        if active.room != Some(ev.room) {
            continue;
        }

        if matches!(
            active.event_type,
            Some(EventType::TimedChallenge | EventType::EliteEncounter)
        ) && active.combat_reward_ready
        {
            if let Some(augment_id) = pick_random_augment_id(
                data.as_deref(),
                &mut rng,
                AugmentPool::EliteOnly,
            ) {
                if let Ok(mut inventory) = player_q.get_single_mut() {
                    inventory.add(augment_id);
                }
            }
        }

        mark_event_resolved(&mut active);
    }
}

fn configure_non_combat_event(
    active: &mut ActiveEvent,
    data: Option<&GameDataRegistry>,
    rng: &mut GameRng,
) {
    let Some(event_type) = active.event_type else {
        return;
    };

    match event_type {
        EventType::Gambler => {
            let Some(augment_id) = pick_random_augment_id(data, rng, AugmentPool::Any) else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            active.choices = vec![
                EventChoice {
                    label: "试试手气".to_string(),
                    description: "支付 50 金币，随机获得一项强化。".to_string(),
                },
                leave_choice(),
            ];
            active.choice_payloads = vec![
                EventChoicePayload::Gambler {
                    cost: 50,
                    augment_id,
                },
                EventChoicePayload::Leave,
            ];
        }
        EventType::CurseAltar => {
            let Some(data) = data else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            let Some((curse_id, duration, curse_title)) = pick_random_curse(data, rng) else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            let Some((augment_id, augment_title, _)) =
                pick_random_augment_offer(data, rng, AugmentPool::EliteOnly)
            else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            active.choices = vec![
                EventChoice {
                    label: "接受代价".to_string(),
                    description: format!("获得诅咒“{curse_title}”，并立刻得到强化“{augment_title}”。"),
                },
                leave_choice(),
            ];
            active.choice_payloads = vec![
                EventChoicePayload::CurseAltar {
                    curse_id,
                    duration,
                    augment_id,
                },
                EventChoicePayload::Leave,
            ];
        }
        EventType::BloodPact => {
            let Some(data) = data else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            let offers = pick_augment_offers(data, rng, AugmentPool::Any, 2);
            if offers.is_empty() {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            }
            active.choices = offers
                .iter()
                .map(|(_, title, description)| EventChoice {
                    label: title.clone(),
                    description: format!("失去 30% 当前生命，获得强化。{description}"),
                })
                .collect();
            active.choice_payloads = offers
                .into_iter()
                .map(|(augment_id, _, _)| EventChoicePayload::BloodPact { augment_id })
                .collect();
        }
        EventType::Treasure => {
            let Some(data) = data else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            let offers = pick_augment_offers(data, rng, AugmentPool::CommonOnly, 2);
            if offers.is_empty() {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            }
            active.choices = offers
                .iter()
                .map(|(_, title, description)| EventChoice {
                    label: title.clone(),
                    description: format!("免费获得强化并额外得到 30 金币。{description}"),
                })
                .collect();
            active.choice_payloads = offers
                .into_iter()
                .map(|(augment_id, _, _)| EventChoicePayload::Treasure {
                    augment_id,
                    gold_bonus: 30,
                })
                .collect();
        }
        EventType::HealingSpring => {
            active.choices = vec![
                EventChoice {
                    label: "饮用泉水".to_string(),
                    description: "恢复 40% 最大生命值。".to_string(),
                },
                leave_choice(),
            ];
            active.choice_payloads = vec![
                EventChoicePayload::HealingSpring,
                EventChoicePayload::Leave,
            ];
        }
        EventType::Merchant => {
            let Some(data) = data else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            let offers = pick_augment_offers(data, rng, AugmentPool::Any, 2);
            if offers.is_empty() {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            }
            active.choices = offers
                .iter()
                .map(|(augment_id, title, description)| {
                    let cost = half_price_augment_cost(data, *augment_id);
                    EventChoice {
                        label: format!("{title} - {cost} 金币"),
                        description: format!("半价购入一项强化。{description}"),
                    }
                })
                .collect();
            active.choice_payloads = offers
                .into_iter()
                .map(|(augment_id, _, _)| EventChoicePayload::Merchant {
                    cost: half_price_augment_cost(data, augment_id),
                    augment_id,
                })
                .collect();
        }
        EventType::PressurePlate
        | EventType::SwitchOrder
        | EventType::TrapSurvival
        | EventType::TimedChallenge
        | EventType::EliteEncounter => {}
    }
}

fn spawn_elite_event_enemy(
    commands: &mut Commands,
    assets: &GameAssets,
    data: &GameDataRegistry,
    rng: &mut GameRng,
    floor_number: u32,
    floor_multiplier: f32,
) {
    let mut pool = spawner::choose_enemy_types(data, floor_number);
    pool.retain(|enemy_type| *enemy_type != EnemyType::Boss);
    let enemy_type = spawner::pick_enemy_type(rng, &pool);
    let pos = spawner::get_spawn_points_for_room()
        .into_iter()
        .next()
        .unwrap_or(Vec2::ZERO);
    spawn_enemy(
        commands,
        assets,
        data,
        enemy_type,
        pos,
        floor_number,
        floor_multiplier,
        1.0,
        true,
    );
}

fn pick_weighted_event(rng: &mut GameRng) -> EventType {
    let weighted = [
        (EventType::PressurePlate, 1_u32),
        (EventType::SwitchOrder, 1),
        (EventType::TrapSurvival, 1),
        (EventType::Gambler, 2),
        (EventType::CurseAltar, 2),
        (EventType::BloodPact, 2),
        (EventType::Treasure, 2),
        (EventType::HealingSpring, 2),
        (EventType::Merchant, 2),
        (EventType::TimedChallenge, 1),
        (EventType::EliteEncounter, 1),
    ];
    let total_weight = weighted.iter().map(|(_, weight)| *weight).sum::<u32>().max(1);
    let mut pick = rng.gen_range_f32(0.0, total_weight as f32).floor() as u32;
    for (event_type, weight) in weighted {
        if pick < weight {
            return event_type;
        }
        pick = pick.saturating_sub(weight);
    }
    EventType::PressurePlate
}

#[derive(Debug, Clone, Copy)]
enum AugmentPool {
    Any,
    CommonOnly,
    EliteOnly,
}

fn pick_random_augment_id(
    data: Option<&GameDataRegistry>,
    rng: &mut GameRng,
    pool: AugmentPool,
) -> Option<AugmentId> {
    pick_random_augment_offer(data?, rng, pool).map(|(augment_id, _, _)| augment_id)
}

fn pick_random_augment_offer(
    data: &GameDataRegistry,
    rng: &mut GameRng,
    pool: AugmentPool,
) -> Option<(AugmentId, String, String)> {
    pick_augment_offers(data, rng, pool, 1).into_iter().next()
}

fn pick_augment_offers(
    data: &GameDataRegistry,
    rng: &mut GameRng,
    pool: AugmentPool,
    count: usize,
) -> Vec<(AugmentId, String, String)> {
    let mut candidates = data
        .augments
        .augments
        .iter()
        .filter(|augment| match pool {
            AugmentPool::Any => true,
            AugmentPool::CommonOnly => augment.rarity == AugmentRarity::Common,
            AugmentPool::EliteOnly => matches!(
                augment.rarity,
                AugmentRarity::Elite | AugmentRarity::Legendary
            ),
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        candidates = data.augments.augments.iter().collect::<Vec<_>>();
    }
    rng.shuffle(&mut candidates);
    let take_count = count.min(candidates.len());
    candidates
        .into_iter()
        .take(take_count)
        .map(|augment| {
            (
                augment.id,
                augment.title.clone(),
                augment.description.clone(),
            )
        })
        .collect()
}

fn pick_random_curse(
    data: &GameDataRegistry,
    rng: &mut GameRng,
) -> Option<(CurseId, u32, String)> {
    let mut curses = data.curses.curses.iter().collect::<Vec<_>>();
    if curses.is_empty() {
        return None;
    }
    rng.shuffle(&mut curses);
    curses.into_iter().next().map(|curse| {
        (
            curse.id,
            curse.duration,
            curse.title.clone(),
        )
    })
}

fn half_price_augment_cost(data: &GameDataRegistry, augment_id: AugmentId) -> u32 {
    data.augments
        .augments
        .iter()
        .find(|augment| augment.id == augment_id)
        .map(|augment| {
            let base_cost = if augment.shop_cost > 0 {
                augment.shop_cost
            } else {
                match augment.rarity {
                    AugmentRarity::Common => 40,
                    AugmentRarity::Elite => 70,
                    AugmentRarity::Legendary => 120,
                }
            };
            (base_cost.max(1) / 2).max(1)
        })
        .unwrap_or(20)
}

fn leave_choice() -> EventChoice {
    EventChoice {
        label: "离开".to_string(),
        description: "放弃这次事件，立即返回地图。".to_string(),
    }
}

enum EventInputOutcome {
    StayOpen,
    Leave,
    Complete,
}

fn apply_choice_payload(
    payload: EventChoicePayload,
    gold: &mut Gold,
    health: &mut Health,
    inventory: &mut AugmentInventory,
    curses: &mut CurseState,
) -> EventInputOutcome {
    match payload {
        EventChoicePayload::Leave => EventInputOutcome::Leave,
        EventChoicePayload::Gambler { cost, augment_id } => {
            if gold.0 < cost {
                warn!("金币不足：需要 {}，当前 {}", cost, gold.0);
                return EventInputOutcome::StayOpen;
            }
            gold.0 -= cost;
            inventory.add(augment_id);
            EventInputOutcome::Complete
        }
        EventChoicePayload::CurseAltar {
            curse_id,
            duration,
            augment_id,
        } => {
            curses.add_curse(curse_id, duration);
            inventory.add(augment_id);
            EventInputOutcome::Complete
        }
        EventChoicePayload::BloodPact { augment_id } => {
            health.current = (health.current * 0.7).clamp(0.0, health.max);
            inventory.add(augment_id);
            EventInputOutcome::Complete
        }
        EventChoicePayload::Treasure {
            augment_id,
            gold_bonus,
        } => {
            inventory.add(augment_id);
            gold.0 = gold.0.saturating_add(gold_bonus);
            EventInputOutcome::Complete
        }
        EventChoicePayload::HealingSpring => {
            health.current = (health.current + health.max * 0.4).min(health.max);
            EventInputOutcome::Complete
        }
        EventChoicePayload::Merchant { cost, augment_id } => {
            if gold.0 < cost {
                warn!("金币不足：需要 {}，当前 {}", cost, gold.0);
                return EventInputOutcome::StayOpen;
            }
            gold.0 -= cost;
            inventory.add(augment_id);
            EventInputOutcome::Complete
        }
    }
}
