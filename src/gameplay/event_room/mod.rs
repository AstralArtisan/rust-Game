use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::core::events::RoomClearedEvent;
use crate::core::input::PlayerInputState;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory, AugmentRarity};
use crate::gameplay::enemy::components::EnemyType;
use crate::gameplay::enemy::spawner;
use crate::gameplay::enemy::systems::{spawn_enemy, spawn_room_enemies};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId, RoomType};
use crate::gameplay::map::transitions::RoomTransition;
use crate::gameplay::player::components::{Gold, Health, Player};
use crate::gameplay::progression::difficulty::{
    get_floor_difficulty_multiplier, get_floor_enemy_count,
};
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::puzzle;
use crate::states::{AppState, GamePhase, RoomState};
use crate::utils::rng::GameRng;

pub struct EventRoomPlugin;

impl Plugin for EventRoomPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveEvent>()
            .add_systems(
                Update,
                (
                    init_event_for_room
                        .after(crate::gameplay::enemy::systems::room_entry_spawner),
                    sync_event_interact_prompt.after(init_event_for_room),
                    event_interact_system.after(sync_event_interact_prompt),
                )
                    .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
            )
            .add_systems(
                Update,
                tick_timed_challenge
                    .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
            )
            .add_systems(
                Update,
                event_room_input.run_if(in_state(GamePhase::EventRoom)),
            )
            .add_systems(
                Update,
                resolve_event_room_clear.run_if(
                    in_state(AppState::InGame)
                        .or_else(in_state(AppState::CoopGame))
                        .and_then(in_state(GamePhase::Playing)),
                ),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    PressurePlate,
    SwitchOrder,
    TrapSurvival,
    Gambler,
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
            Self::BloodPact => "献出生命，换取一项强化。",
            Self::Treasure => "挑选宝藏并顺手带走金币。",
            Self::HealingSpring => "停留片刻，恢复生命。",
            Self::Merchant => "挑一件半价强化带走。",
            Self::TimedChallenge => "30 秒内清空房间，可得精英强化。",
            Self::EliteEncounter => "击败单个精英，直接获得精英强化。",
        }
    }

    pub fn accent_color(self) -> Color {
        match self {
            Self::Gambler => Color::srgb(0.94, 0.76, 0.28),
            Self::BloodPact => Color::srgb(0.88, 0.30, 0.34),
            Self::Treasure => Color::srgb(0.30, 0.82, 0.54),
            Self::HealingSpring => Color::srgb(0.28, 0.72, 0.96),
            Self::Merchant => Color::srgb(0.94, 0.56, 0.24),
            Self::PressurePlate | Self::SwitchOrder | Self::TrapSurvival => {
                Color::srgb(0.82, 0.82, 0.88)
            }
            Self::TimedChallenge | Self::EliteEncounter => Color::srgb(0.96, 0.42, 0.24),
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Gambler => "◈",
            Self::BloodPact => "♦",
            Self::Treasure => "✦",
            Self::HealingSpring => "✿",
            Self::Merchant => "⚙",
            Self::PressurePlate | Self::SwitchOrder | Self::TrapSurvival => "⌘",
            Self::TimedChallenge | Self::EliteEncounter => "⚔",
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
    Gambler {
        cost: u32,
        augment_id: AugmentId,
    },
    BloodPact {
        augment_id: AugmentId,
    },
    Treasure {
        augment_id: AugmentId,
        gold_bonus: u32,
    },
    HealingSpring,
    Merchant {
        cost: u32,
        augment_id: AugmentId,
    },
}

#[derive(Resource, Debug, Clone)]
pub struct ActiveEvent {
    pub event_type: Option<EventType>,
    pub resolved: bool,
    pub room: Option<RoomId>,
    pub interaction_ready: bool,
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
            interaction_ready: false,
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
    active.event_type = None;
    active.interaction_ready = false;
    active.choices.clear();
    active.choice_payloads.clear();
    active.combat_reward_ready = false;
    active.timed_challenge_timer = None;
}

#[derive(Component)]
pub struct EventInteractPrompt;

fn init_event_for_room(
    layout: Option<Res<FloorLayout>>,
    current_room: Option<Res<CurrentRoom>>,
    mut active: ResMut<ActiveEvent>,
    mut rng: ResMut<GameRng>,
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
    if active.room == Some(current_room.0) {
        return;
    }

    reset_active_event(&mut active);
    active.room = Some(current_room.0);
    active.event_type = Some(pick_weighted_event(&mut rng));
    active.interaction_ready = true;
}

fn sync_event_interact_prompt(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    layout: Option<Res<FloorLayout>>,
    current_room: Option<Res<CurrentRoom>>,
    active: Res<ActiveEvent>,
    prompt_q: Query<Entity, With<EventInteractPrompt>>,
) {
    let should_show = layout
        .as_deref()
        .zip(current_room.as_deref())
        .and_then(|(layout, current_room)| {
            let room = layout.room(current_room.0)?;
            Some(room.room_type == RoomType::Event && active.room == Some(current_room.0))
        })
        .unwrap_or(false)
        && active.interaction_ready
        && !active.resolved;

    if !should_show {
        for entity in &prompt_q {
            commands.entity(entity).despawn_recursive();
        }
        return;
    }

    if prompt_q.iter().next().is_some() {
        return;
    }

    let (Some(assets), Some(event_type)) = (assets, active.event_type) else {
        return;
    };
    spawn_event_interact_prompt(&mut commands, &assets, event_type);
}

fn spawn_event_interact_prompt(
    commands: &mut Commands,
    assets: &GameAssets,
    event_type: EventType,
) {
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                format!("【{}】\n按 E 交互", event_type.title()),
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 20.0,
                    color: event_type.accent_color(),
                },
            )
            .with_justify(JustifyText::Center),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
            ..default()
        },
        EventInteractPrompt,
        InGameEntity,
        Name::new("EventInteractPrompt"),
    ));
}

fn event_interact_system(
    input: Res<PlayerInputState>,
    mut active: ResMut<ActiveEvent>,
    mut next_state: ResMut<NextState<GamePhase>>,
    assets: Res<GameAssets>,
    data: Option<Res<GameDataRegistry>>,
    layout: Option<Res<FloorLayout>>,
    current_room: Option<Res<CurrentRoom>>,
    transition: Option<Res<RoomTransition>>,
    mut room_state: ResMut<RoomState>,
    mut active_puzzle: ResMut<puzzle::ActivePuzzle>,
    mut rng: ResMut<GameRng>,
    floor: Option<Res<FloorNumber>>,
    player_q: Query<&GlobalTransform, With<Player>>,
    prompt_q: Query<(Entity, &GlobalTransform), With<EventInteractPrompt>>,
    mut commands: Commands,
) {
    if !input.interact_pressed
        || transition
            .as_deref()
            .map(|value| value.active)
            .unwrap_or(false)
    {
        return;
    }
    if !active.interaction_ready || active.resolved {
        return;
    }

    let (Some(layout), Some(current_room)) = (layout.as_deref(), current_room.as_deref()) else {
        return;
    };
    let Some(room) = layout.room(current_room.0) else {
        return;
    };
    if room.room_type != RoomType::Event || active.room != Some(current_room.0) {
        return;
    }

    let Ok(player_transform) = player_q.get_single() else {
        return;
    };
    let player_pos = player_transform.translation().truncate();
    let can_interact = prompt_q.iter().any(|(_, prompt_transform)| {
        player_pos.distance(prompt_transform.translation().truncate()) <= 80.0
    });
    if !can_interact {
        return;
    }

    for (entity, _) in &prompt_q {
        commands.entity(entity).despawn_recursive();
    }
    active.interaction_ready = false;

    let Some(event_type) = active.event_type else {
        return;
    };

    // Re-interaction with an already-configured non-combat event (the player
    // pressed Esc to back out): re-open the same menu WITHOUT re-rolling its
    // contents. Esc deliberately does not resolve the event, but without this
    // guard each Esc+E would re-roll Gambler/Treasure/BloodPact rewards for
    // free until a favorable roll appeared.
    if !event_type.is_combat() && !event_type.is_puzzle() && !active.choice_payloads.is_empty() {
        next_state.set(GamePhase::EventRoom);
        return;
    }

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
            active.timed_challenge_timer = Some(Timer::from_seconds(30.0, TimerMode::Once));
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
        | EventType::BloodPact
        | EventType::Treasure
        | EventType::HealingSpring
        | EventType::Merchant => {
            configure_non_combat_event(&mut active, data.as_deref(), &mut rng);
            *room_state = RoomState::Locked;
            next_state.set(GamePhase::EventRoom);
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
    mut next_state: ResMut<NextState<GamePhase>>,
    mut cleared: EventWriter<RoomClearedEvent>,
    mut player_q: Query<(&mut Gold, &mut Health, &mut AugmentInventory), With<Player>>,
) {
    let Some(room) = active.room else {
        next_state.set(GamePhase::Playing);
        return;
    };

    if active.resolved {
        next_state.set(GamePhase::Playing);
        return;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        active.interaction_ready = true;
        next_state.set(GamePhase::Playing);
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
    let Ok((mut gold, mut health, mut inventory)) = player_q.get_single_mut() else {
        return;
    };

    match apply_choice_payload(payload, &mut gold, &mut health, &mut inventory) {
        EventInputOutcome::StayOpen => {}
        EventInputOutcome::Leave => {
            *room_state = RoomState::Cleared;
            mark_event_resolved(&mut active);
            next_state.set(GamePhase::Playing);
        }
        EventInputOutcome::Complete => {
            *room_state = RoomState::Cleared;
            mark_event_resolved(&mut active);
            cleared.send(RoomClearedEvent { room });
            next_state.set(GamePhase::Playing);
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

        let should_reward = match active.event_type {
            Some(EventType::TimedChallenge | EventType::EliteEncounter) => {
                active.combat_reward_ready
            }
            Some(event_type) if event_type.is_puzzle() => true,
            _ => false,
        };

        if should_reward {
            let pool = if active.event_type.is_some_and(EventType::is_puzzle) {
                AugmentPool::Any
            } else {
                AugmentPool::EliteOnly
            };
            if let Some(augment_id) = pick_random_augment_id(data.as_deref(), &mut rng, pool) {
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
            active.choice_payloads =
                vec![EventChoicePayload::HealingSpring, EventChoicePayload::Leave];
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
        (EventType::BloodPact, 2),
        (EventType::Treasure, 2),
        (EventType::HealingSpring, 2),
        (EventType::Merchant, 2),
        (EventType::TimedChallenge, 1),
        (EventType::EliteEncounter, 1),
    ];
    let total_weight = weighted
        .iter()
        .map(|(_, weight)| *weight)
        .sum::<u32>()
        .max(1);
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
