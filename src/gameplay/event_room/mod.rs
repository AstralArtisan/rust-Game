use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::core::events::{DamageAppliedEvent, RoomClearedEvent};
use crate::core::input::PlayerInputState;
use crate::data::definitions::{AugmentConfig, PuzzleEventConfig, PuzzleRewardPool};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{
    AugmentGrantResult, AugmentId, AugmentInventory, AugmentRarity,
};
use crate::gameplay::combat::components::Team;
use crate::gameplay::enemy::components::EnemyType;
use crate::gameplay::enemy::spawner;
use crate::gameplay::enemy::systems::{spawn_enemy, spawn_room_enemies};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId, RoomType};
use crate::gameplay::map::transitions::RoomTransition;
use crate::gameplay::player::components::{Energy, Gold, Health, Player};
use crate::gameplay::progression::difficulty::{
    get_floor_difficulty_multiplier, get_floor_enemy_count,
};
use crate::gameplay::progression::experience::PlayerLevel;
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::puzzle::{self, PuzzleKind};
use crate::states::{AppState, GamePhase, RoomState};
use crate::ui::feedback::{UiFeedbackEvent, UiFeedbackSeverity};
use crate::utils::rng::GameRng;

pub struct EventRoomPlugin;

impl Plugin for EventRoomPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveEvent>()
            .init_resource::<EventPendingAction>()
            .add_systems(
                Update,
                (
                    init_event_for_room.after(crate::gameplay::enemy::systems::room_entry_spawner),
                    sync_event_interact_prompt.after(init_event_for_room),
                    event_interact_system.after(sync_event_interact_prompt),
                )
                    .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
            )
            .add_systems(
                Update,
                (tick_timed_challenge, track_flawless_trial_damage)
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
    BulletMaze,
    MemoryBlocks,
    TimedCollect,
    Gambler,
    BloodPact,
    Treasure,
    HealingSpring,
    Merchant,
    GoldAltar,
    MysticBlacksmith,
    WheelOfFate,
    TravelerGift,
    SacrificeAltar,
    TimedChallenge,
    EliteEncounter,
    FlawlessTrial,
    CursedVault,
}

impl EventType {
    pub const PHASE3_EVENTS: [Self; 17] = [
        Self::BulletMaze,
        Self::MemoryBlocks,
        Self::TimedCollect,
        Self::Gambler,
        Self::BloodPact,
        Self::Treasure,
        Self::HealingSpring,
        Self::Merchant,
        Self::GoldAltar,
        Self::MysticBlacksmith,
        Self::WheelOfFate,
        Self::TravelerGift,
        Self::SacrificeAltar,
        Self::TimedChallenge,
        Self::EliteEncounter,
        Self::FlawlessTrial,
        Self::CursedVault,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::BulletMaze => "弹幕迷宫",
            Self::MemoryBlocks => "记忆方块",
            Self::TimedCollect => "限时收集",
            Self::Gambler => "赌徒",
            Self::BloodPact => "血契",
            Self::Treasure => "宝箱",
            Self::HealingSpring => "治愈泉",
            Self::Merchant => "流动商贩",
            Self::GoldAltar => "金币祭坛",
            Self::MysticBlacksmith => "神秘铸炉",
            Self::WheelOfFate => "命运之轮",
            Self::TravelerGift => "旅者休憩",
            Self::SacrificeAltar => "献祭祭坛",
            Self::TimedChallenge => "限时清剿",
            Self::EliteEncounter => "猎杀挑战",
            Self::FlawlessTrial => "无伤试炼",
            Self::CursedVault => "诅咒宝库",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::BulletMaze => "穿过移动弹幕墙抵达终点。",
            Self::MemoryBlocks => "记住亮起顺序并复现。",
            Self::TimedCollect => "在时间压力下收集闪烁能量球。",
            Self::Gambler => "付出金币，赌一项随机强化。",
            Self::BloodPact => "献出生命，换取一项强化。",
            Self::Treasure => "打开免费宝箱，获得强化与金币。",
            Self::HealingSpring => "恢复生命、能量，或各恢复一部分。",
            Self::Merchant => "挑一件半价强化带走。",
            Self::GoldAltar => "用生命或能量换取金币。",
            Self::MysticBlacksmith => "献祭已有强化，获得同类别更高稀有度强化。",
            Self::WheelOfFate => "转动命运，接受随机奖惩。",
            Self::TravelerGift => "休息、交谈或交易一项强化。",
            Self::SacrificeAltar => "永久献出最大生命，换取传说强化。",
            Self::TimedChallenge => "30 秒内清空房间，可得精英强化。",
            Self::EliteEncounter => "击败多词缀精英，获得精英强化与金币。",
            Self::FlawlessTrial => "无伤清敌可获得传说强化。",
            Self::CursedVault => "连续开箱，金币与伏击并存。",
        }
    }

    pub fn accent_color(self) -> Color {
        match self {
            Self::Gambler => Color::srgb(0.94, 0.76, 0.28),
            Self::BloodPact => Color::srgb(0.88, 0.30, 0.34),
            Self::Treasure => Color::srgb(0.30, 0.82, 0.54),
            Self::HealingSpring => Color::srgb(0.28, 0.72, 0.96),
            Self::Merchant => Color::srgb(0.94, 0.56, 0.24),
            Self::GoldAltar | Self::WheelOfFate | Self::TravelerGift => {
                Color::srgb(0.92, 0.78, 0.30)
            }
            Self::MysticBlacksmith | Self::SacrificeAltar => Color::srgb(0.78, 0.32, 0.36),
            Self::BulletMaze | Self::MemoryBlocks | Self::TimedCollect => {
                Color::srgb(0.82, 0.82, 0.88)
            }
            Self::TimedChallenge
            | Self::EliteEncounter
            | Self::FlawlessTrial
            | Self::CursedVault => Color::srgb(0.96, 0.42, 0.24),
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Gambler => "◈",
            Self::BloodPact => "♦",
            Self::Treasure => "✦",
            Self::HealingSpring => "✿",
            Self::Merchant => "⚙",
            Self::GoldAltar | Self::WheelOfFate | Self::TravelerGift => "◉",
            Self::MysticBlacksmith | Self::SacrificeAltar => "◆",
            Self::BulletMaze | Self::MemoryBlocks | Self::TimedCollect => "⌘",
            Self::TimedChallenge
            | Self::EliteEncounter
            | Self::FlawlessTrial
            | Self::CursedVault => "⚔",
        }
    }

    pub fn is_puzzle(self) -> bool {
        matches!(
            self,
            Self::BulletMaze | Self::MemoryBlocks | Self::TimedCollect
        )
    }

    pub fn puzzle_kind(self) -> Option<PuzzleKind> {
        match self {
            Self::BulletMaze => Some(PuzzleKind::BulletMaze),
            Self::MemoryBlocks => Some(PuzzleKind::MemoryBlocks),
            Self::TimedCollect => Some(PuzzleKind::TimedCollect),
            _ => None,
        }
    }

    pub fn config_id(self) -> &'static str {
        match self {
            Self::BulletMaze => "bullet_maze",
            Self::MemoryBlocks => "memory_blocks",
            Self::TimedCollect => "timed_collect",
            Self::Gambler => "gambler",
            Self::BloodPact => "blood_pact",
            Self::Treasure => "treasure",
            Self::HealingSpring => "healing_spring",
            Self::Merchant => "traveling_merchant",
            Self::GoldAltar => "gold_altar",
            Self::MysticBlacksmith => "mystic_blacksmith",
            Self::WheelOfFate => "wheel_of_fate",
            Self::TravelerGift => "traveler_gift",
            Self::SacrificeAltar => "sacrifice_altar",
            Self::TimedChallenge => "timed_challenge",
            Self::EliteEncounter => "hunt_challenge",
            Self::FlawlessTrial => "flawless_trial",
            Self::CursedVault => "cursed_vault",
        }
    }

    fn is_combat(self) -> bool {
        matches!(
            self,
            Self::TimedChallenge | Self::EliteEncounter | Self::FlawlessTrial | Self::CursedVault
        )
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
    Unavailable {
        title: String,
        message: String,
    },
    Gambler {
        cost: u32,
        augment_id: AugmentId,
    },
    BloodPact {
        hp_fraction: f32,
        augment_id: AugmentId,
    },
    Treasure {
        augment_id: AugmentId,
        gold_bonus: u32,
    },
    HealingSpring {
        hp_fraction: f32,
        energy_flat: f32,
        energy_fraction: f32,
    },
    Merchant {
        cost: u32,
        augment_id: AugmentId,
    },
    GoldAltar {
        gold: u32,
        hp_fraction: f32,
        energy_cost: f32,
    },
    MysticForge {
        sacrifice_id: AugmentId,
        reward_id: AugmentId,
    },
    WheelOfFate,
    TravelerRest,
    TravelerIntel {
        lines: Vec<String>,
    },
    TravelerTrade {
        sacrifice_id: AugmentId,
        gold: u32,
    },
    SacrificeAltar {
        augment_id: AugmentId,
    },
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ActiveEvent {
    pub event_type: Option<EventType>,
    pub resolved: bool,
    pub room: Option<RoomId>,
    pub interaction_ready: bool,
    pub choices: Vec<EventChoice>,
    choice_payloads: Vec<EventChoicePayload>,
    combat_reward_ready: bool,
    flawless_failed: bool,
    timed_challenge_timer: Option<Timer>,
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct EventPendingAction(pub Option<EventUiAction>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventUiAction {
    Select(usize),
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
    active.flawless_failed = false;
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
    player_q: Query<(&GlobalTransform, Option<&AugmentInventory>), With<Player>>,
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

    let Ok((player_transform, inventory)) = player_q.get_single() else {
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
        EventType::BulletMaze | EventType::MemoryBlocks | EventType::TimedCollect => {
            let Some(kind) = event_type.puzzle_kind() else {
                return;
            };
            let Some(config) = puzzle_config_for_event(data.as_deref(), event_type) else {
                mark_event_resolved(&mut active);
                return;
            };
            *room_state = RoomState::Locked;
            puzzle::spawn_puzzle_for_kind(
                &mut commands,
                &assets,
                &mut active_puzzle,
                current_room.0,
                kind,
                config,
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
        EventType::FlawlessTrial => {
            let Some(data) = data.as_deref() else {
                mark_event_resolved(&mut active);
                return;
            };
            *room_state = RoomState::Locked;
            active.combat_reward_ready = true;
            active.flawless_failed = false;
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
        EventType::CursedVault => {
            let Some(data) = data.as_deref() else {
                mark_event_resolved(&mut active);
                return;
            };
            *room_state = RoomState::Locked;
            active.combat_reward_ready = true;
            let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
            let enemy_count = get_floor_enemy_count(data, floor_number)
                .saturating_add(2)
                .max(5);
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
            spawn_hunt_challenge_enemies(
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
        | EventType::Merchant
        | EventType::GoldAltar
        | EventType::MysticBlacksmith
        | EventType::WheelOfFate
        | EventType::TravelerGift
        | EventType::SacrificeAltar => {
            configure_non_combat_event(
                &mut active,
                data.as_deref(),
                &mut rng,
                inventory,
                layout,
                current_room.0,
            );
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

fn track_flawless_trial_damage(
    mut damage: EventReader<DamageAppliedEvent>,
    mut active: ResMut<ActiveEvent>,
) {
    if active.event_type != Some(EventType::FlawlessTrial) || active.resolved {
        damage.clear();
        return;
    }

    if damage.read().any(|event| {
        event.amount > 0.0
            && event.attacker_team == Team::Enemy
            && event.target_team == Some(Team::Player)
    }) {
        active.flawless_failed = true;
        active.combat_reward_ready = false;
    }
}

fn event_room_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut pending_action: ResMut<EventPendingAction>,
    mut active: ResMut<ActiveEvent>,
    mut room_state: ResMut<RoomState>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut cleared: EventWriter<RoomClearedEvent>,
    mut feedback: EventWriter<UiFeedbackEvent>,
    data: Option<Res<GameDataRegistry>>,
    mut rng: ResMut<GameRng>,
    mut player_q: Query<
        (
            &mut Gold,
            &mut Health,
            &mut Energy,
            &mut AugmentInventory,
            &mut PlayerLevel,
        ),
        With<Player>,
    >,
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

    let index = match pending_action.0.take() {
        Some(EventUiAction::Select(index)) => Some(index),
        None => {
            if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1) {
                Some(0)
            } else if keyboard.just_pressed(KeyCode::Digit2)
                || keyboard.just_pressed(KeyCode::Numpad2)
            {
                Some(1)
            } else if keyboard.just_pressed(KeyCode::Digit3)
                || keyboard.just_pressed(KeyCode::Numpad3)
            {
                Some(2)
            } else if keyboard.just_pressed(KeyCode::Digit4)
                || keyboard.just_pressed(KeyCode::Numpad4)
            {
                Some(3)
            } else {
                None
            }
        }
    };
    let Some(index) = index else {
        return;
    };
    let Some(payload) = active.choice_payloads.get(index).cloned() else {
        return;
    };
    let Ok((mut gold, mut health, mut energy, mut inventory, mut level)) =
        player_q.get_single_mut()
    else {
        return;
    };

    let result = apply_choice_payload(
        payload,
        &mut gold,
        &mut health,
        &mut energy,
        &mut inventory,
        &mut level,
        data.as_deref(),
        &mut rng,
    );
    if let Some(event) = result.feedback {
        feedback.send(event);
    }
    match result.outcome {
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
    mut active_puzzle: ResMut<puzzle::ActivePuzzle>,
    data: Option<Res<GameDataRegistry>>,
    mut rng: ResMut<GameRng>,
    mut feedback: EventWriter<UiFeedbackEvent>,
    mut player_q: Query<(&mut AugmentInventory, &mut Gold, &mut PlayerLevel), With<Player>>,
) {
    for ev in cleared.read() {
        if active.room != Some(ev.room) {
            continue;
        }

        match active.event_type {
            Some(event_type) if event_type.is_puzzle() => {
                apply_puzzle_event_reward(
                    &mut active_puzzle,
                    data.as_deref(),
                    &mut rng,
                    &mut feedback,
                    &mut player_q,
                );
                puzzle::reset_active_puzzle(&mut active_puzzle);
            }
            Some(event_type) if event_type.is_combat() => {
                apply_combat_event_reward(
                    event_type,
                    active.combat_reward_ready,
                    active.flawless_failed,
                    data.as_deref(),
                    &mut rng,
                    &mut feedback,
                    &mut player_q,
                );
            }
            _ => {}
        }

        mark_event_resolved(&mut active);
    }
}

fn apply_puzzle_event_reward(
    active_puzzle: &mut puzzle::ActivePuzzle,
    data: Option<&GameDataRegistry>,
    rng: &mut GameRng,
    feedback: &mut EventWriter<UiFeedbackEvent>,
    player_q: &mut Query<(&mut AugmentInventory, &mut Gold, &mut PlayerLevel), With<Player>>,
) {
    if !active_puzzle.reward_earned {
        feedback.send(UiFeedbackEvent {
            title: "谜题失败".to_string(),
            lines: vec!["没有获得宝箱奖励。".to_string()],
            severity: UiFeedbackSeverity::Warning,
            requires_ack: false,
            return_phase: GamePhase::Playing,
        });
        return;
    }

    let reward = active_puzzle.reward_to_apply();
    let mut lines = Vec::new();
    if let Ok((mut inventory, mut gold, mut level)) = player_q.get_single_mut() {
        if reward.gold > 0 {
            gold.0 = gold.0.saturating_add(reward.gold);
            lines.push(format!("+{} 金币", reward.gold));
        }
        if reward.xp > 0 {
            let levels_gained = level.add_xp(reward.xp);
            lines.push(format!("+{} 经验", reward.xp));
            if levels_gained > 0 {
                lines.push(format!("等级提升到 {}", level.level));
            }
        }
        if let Some(pool) = augment_pool_from_puzzle(reward.augment_pool)
            && let Some(augment_id) = pick_random_augment_id(data, rng, pool)
        {
            let grant = inventory.grant(augment_id);
            lines.extend(describe_augment_grant(grant, data));
        }
    }

    if active_puzzle.kind == Some(PuzzleKind::TimedCollect) {
        lines.insert(
            0,
            format!(
                "收集进度：{}/{}",
                active_puzzle.progress_count(),
                active_puzzle.target_count().max(1)
            ),
        );
    }
    if lines.is_empty() {
        lines.push("宝箱里没有可用奖励。".to_string());
    }

    feedback.send(UiFeedbackEvent::card(
        "谜题宝箱",
        lines,
        UiFeedbackSeverity::Success,
        GamePhase::Playing,
    ));
}

fn apply_combat_event_reward(
    event_type: EventType,
    reward_ready: bool,
    flawless_failed: bool,
    data: Option<&GameDataRegistry>,
    rng: &mut GameRng,
    feedback: &mut EventWriter<UiFeedbackEvent>,
    player_q: &mut Query<(&mut AugmentInventory, &mut Gold, &mut PlayerLevel), With<Player>>,
) {
    let (pools, gold_bonus, title) = match event_type {
        EventType::TimedChallenge if reward_ready => {
            (vec![AugmentPool::EliteOnly], 0, "限时清剿达成")
        }
        EventType::TimedChallenge => (vec![AugmentPool::CommonOnly], 0, "限时清剿超时"),
        EventType::EliteEncounter => (vec![AugmentPool::EliteOnly], 40, "猎杀挑战完成"),
        EventType::FlawlessTrial if !flawless_failed => {
            (vec![AugmentPool::LegendaryOnly], 0, "无伤试炼达成")
        }
        EventType::FlawlessTrial => (vec![AugmentPool::CommonOnly], 0, "无伤试炼受伤"),
        EventType::CursedVault => (
            vec![AugmentPool::EliteOnly, AugmentPool::EliteOnly],
            90,
            "诅咒宝库清空",
        ),
        _ => return,
    };

    let mut lines = Vec::new();
    if let Ok((mut inventory, mut gold, _)) = player_q.get_single_mut() {
        if gold_bonus > 0 {
            gold.0 = gold.0.saturating_add(gold_bonus);
            lines.push(format!("+{} 金币", gold_bonus));
        }
        for pool in pools {
            if let Some(augment_id) = pick_random_augment_id(data, rng, pool) {
                let grant = inventory.grant(augment_id);
                lines.extend(describe_augment_grant(grant, data));
            }
        }
    }
    if lines.is_empty() {
        lines.push("没有找到可用强化，已完成事件。".to_string());
    }

    feedback.send(UiFeedbackEvent::card(
        title,
        lines,
        UiFeedbackSeverity::Success,
        GamePhase::Playing,
    ));
}

fn augment_pool_from_puzzle(pool: PuzzleRewardPool) -> Option<AugmentPool> {
    match pool {
        PuzzleRewardPool::None => None,
        PuzzleRewardPool::Any => Some(AugmentPool::Any),
        PuzzleRewardPool::Elite => Some(AugmentPool::EliteOnly),
        PuzzleRewardPool::Legendary => Some(AugmentPool::LegendaryOnly),
    }
}

fn configure_non_combat_event(
    active: &mut ActiveEvent,
    data: Option<&GameDataRegistry>,
    rng: &mut GameRng,
    inventory: Option<&AugmentInventory>,
    layout: &FloorLayout,
    current_room: RoomId,
) {
    let Some(event_type) = active.event_type else {
        return;
    };

    match event_type {
        EventType::Gambler => {
            let Some(lite_augment_id) = pick_random_augment_id(data, rng, AugmentPool::Any) else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            let Some(elite_augment_id) = pick_random_augment_id(data, rng, AugmentPool::EliteOnly)
            else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            active.choices = vec![
                EventChoice {
                    label: "50 金币赌一把".to_string(),
                    description: "支付 50 金币，随机获得一项强化。".to_string(),
                },
                EventChoice {
                    label: "100 金币赌精英".to_string(),
                    description: "支付 100 金币，随机获得一项精英强化。".to_string(),
                },
                leave_choice(),
            ];
            active.choice_payloads = vec![
                EventChoicePayload::Gambler {
                    cost: 50,
                    augment_id: lite_augment_id,
                },
                EventChoicePayload::Gambler {
                    cost: 100,
                    augment_id: elite_augment_id,
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
            let common = pick_random_augment_offer(data, rng, AugmentPool::CommonOnly);
            let elite = pick_random_augment_offer(data, rng, AugmentPool::EliteOnly);
            let (
                Some((common_id, common_title, common_desc)),
                Some((elite_id, elite_title, elite_desc)),
            ) = (common, elite)
            else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            active.choices = vec![
                EventChoice {
                    label: format!("20% 当前 HP：{common_title}"),
                    description: format!("失去 20% 当前生命，获得普通强化。{common_desc}"),
                },
                EventChoice {
                    label: format!("40% 当前 HP：{elite_title}"),
                    description: format!("失去 40% 当前生命，获得精英强化。{elite_desc}"),
                },
                leave_choice(),
            ];
            active.choice_payloads = vec![
                EventChoicePayload::BloodPact {
                    hp_fraction: 0.20,
                    augment_id: common_id,
                },
                EventChoicePayload::BloodPact {
                    hp_fraction: 0.40,
                    augment_id: elite_id,
                },
                EventChoicePayload::Leave,
            ];
        }
        EventType::Treasure => {
            let Some(augment_id) = pick_random_augment_id(data, rng, AugmentPool::Any) else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            let gold_bonus = rng.gen_range_f32(20.0, 41.0).floor() as u32;
            active.choices = vec![EventChoice {
                label: "打开宝箱".to_string(),
                description: format!("免费获得一项强化，并获得 {gold_bonus} 金币。"),
            }];
            active.choice_payloads = vec![EventChoicePayload::Treasure {
                augment_id,
                gold_bonus,
            }];
        }
        EventType::HealingSpring => {
            active.choices = vec![
                EventChoice {
                    label: "恢复 40% HP".to_string(),
                    description: "恢复 40% 最大生命值。".to_string(),
                },
                EventChoice {
                    label: "20% HP + 30 能量".to_string(),
                    description: "恢复 20% 最大生命与 30 点能量。".to_string(),
                },
                EventChoice {
                    label: "两者各恢复 25%".to_string(),
                    description: "恢复 25% 最大生命与 25% 最大能量。".to_string(),
                },
            ];
            active.choice_payloads = vec![
                EventChoicePayload::HealingSpring {
                    hp_fraction: 0.40,
                    energy_flat: 0.0,
                    energy_fraction: 0.0,
                },
                EventChoicePayload::HealingSpring {
                    hp_fraction: 0.20,
                    energy_flat: 30.0,
                    energy_fraction: 0.0,
                },
                EventChoicePayload::HealingSpring {
                    hp_fraction: 0.25,
                    energy_flat: 0.0,
                    energy_fraction: 0.25,
                },
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
                .chain(std::iter::once(leave_choice()))
                .collect();
            active.choice_payloads = offers
                .into_iter()
                .map(|(augment_id, _, _)| EventChoicePayload::Merchant {
                    cost: half_price_augment_cost(data, augment_id),
                    augment_id,
                })
                .chain(std::iter::once(EventChoicePayload::Leave))
                .collect();
        }
        EventType::GoldAltar => {
            let random_hp_fraction = rng.gen_range_f32(0.10, 0.50);
            // design.md §6.1: random gold scales linearly with random HP cost.
            let random_gold = (random_hp_fraction / 0.50 * 80.0).round().max(20.0) as u32;
            active.choices = vec![
                EventChoice {
                    label: "能量换少量金币".to_string(),
                    description: "失去 20 能量，获得 25 金币。".to_string(),
                },
                EventChoice {
                    label: "少量血换少量金币".to_string(),
                    description: "失去 15% 当前生命，获得 30 金币。".to_string(),
                },
                EventChoice {
                    label: "大量血换大量金币".to_string(),
                    description: "失去 50% 当前生命，获得 80 金币。".to_string(),
                },
                EventChoice {
                    label: "随机血换随机金币".to_string(),
                    description: format!(
                        "失去 {:.0}% 当前生命，获得 {} 金币。",
                        random_hp_fraction * 100.0,
                        random_gold
                    ),
                },
            ];
            active.choice_payloads = vec![
                EventChoicePayload::GoldAltar {
                    gold: 25,
                    hp_fraction: 0.0,
                    energy_cost: 20.0,
                },
                EventChoicePayload::GoldAltar {
                    gold: 30,
                    hp_fraction: 0.15,
                    energy_cost: 0.0,
                },
                EventChoicePayload::GoldAltar {
                    gold: 80,
                    hp_fraction: 0.50,
                    energy_cost: 0.0,
                },
                EventChoicePayload::GoldAltar {
                    gold: random_gold,
                    hp_fraction: random_hp_fraction,
                    energy_cost: 0.0,
                },
            ];
        }
        EventType::MysticBlacksmith => {
            let Some(data) = data else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            let offers = build_forge_offers(data, rng, inventory, 3);
            if offers.is_empty() {
                active.choices = vec![
                    EventChoice {
                        label: "无可献祭强化".to_string(),
                        description: "需要至少 1 个可升阶的普通或精英强化。".to_string(),
                    },
                    leave_choice(),
                ];
                active.choice_payloads = vec![
                    EventChoicePayload::Unavailable {
                        title: "神秘铸炉".to_string(),
                        message: "没有可升阶的强化。".to_string(),
                    },
                    EventChoicePayload::Leave,
                ];
                return;
            }
            active.choices = offers
                .iter()
                .map(|offer| EventChoice {
                    label: format!("献祭：{}", offer.sacrifice_title),
                    description: format!(
                        "获得同类别更高稀有度强化：{}。{}",
                        offer.reward_title, offer.reward_description
                    ),
                })
                .chain(std::iter::once(leave_choice()))
                .collect();
            active.choice_payloads = offers
                .into_iter()
                .map(|offer| EventChoicePayload::MysticForge {
                    sacrifice_id: offer.sacrifice_id,
                    reward_id: offer.reward_id,
                })
                .chain(std::iter::once(EventChoicePayload::Leave))
                .collect();
        }
        EventType::WheelOfFate => {
            active.choices = vec![
                EventChoice {
                    label: "转动命运".to_string(),
                    description: "随机触发好/坏各半的结果：金币、等级、强化、生命或失去强化。"
                        .to_string(),
                },
                leave_choice(),
            ];
            active.choice_payloads =
                vec![EventChoicePayload::WheelOfFate, EventChoicePayload::Leave];
        }
        EventType::TravelerGift => {
            let intel = traveler_intel_lines(layout, current_room);
            let trade = inventory.and_then(|inv| inv.augments.first()).map(|held| {
                let gold = trade_gold_for_augment(data, held.id);
                (held.id, gold)
            });
            active.choices = vec![
                EventChoice {
                    label: "休息".to_string(),
                    description: "恢复 20% 最大生命。".to_string(),
                },
                EventChoice {
                    label: "交谈".to_string(),
                    description: "获得后续路线情报：显示前 3 个房间类型。".to_string(),
                },
                EventChoice {
                    label: "交易".to_string(),
                    description: trade
                        .map(|(_, gold)| format!("用 1 个强化换取 {gold} 金币。"))
                        .unwrap_or_else(|| "没有可交易的强化。".to_string()),
                },
            ];
            active.choice_payloads = vec![
                EventChoicePayload::TravelerRest,
                EventChoicePayload::TravelerIntel { lines: intel },
                trade
                    .map(|(sacrifice_id, gold)| EventChoicePayload::TravelerTrade {
                        sacrifice_id,
                        gold,
                    })
                    .unwrap_or_else(|| EventChoicePayload::Unavailable {
                        title: "旅者交易".to_string(),
                        message: "当前没有可交易的强化。".to_string(),
                    }),
            ];
        }
        EventType::SacrificeAltar => {
            let Some(augment_id) = pick_random_augment_id(data, rng, AugmentPool::LegendaryOnly)
            else {
                active.choices = vec![leave_choice()];
                active.choice_payloads = vec![EventChoicePayload::Leave];
                return;
            };
            active.choices = vec![
                EventChoice {
                    label: "失去 20% 最大 HP".to_string(),
                    description: "永久失去 20% 最大生命，获得传说强化。".to_string(),
                },
                leave_choice(),
            ];
            active.choice_payloads = vec![
                EventChoicePayload::SacrificeAltar { augment_id },
                EventChoicePayload::Leave,
            ];
        }
        EventType::BulletMaze
        | EventType::MemoryBlocks
        | EventType::TimedCollect
        | EventType::TimedChallenge
        | EventType::EliteEncounter
        | EventType::FlawlessTrial
        | EventType::CursedVault => {}
    }
}

fn spawn_hunt_challenge_enemies(
    commands: &mut Commands,
    assets: &GameAssets,
    data: &GameDataRegistry,
    rng: &mut GameRng,
    floor_number: u32,
    floor_multiplier: f32,
) {
    let mut pool = spawner::choose_enemy_types(data, floor_number);
    pool.retain(|enemy_type| *enemy_type != EnemyType::Boss);
    let mut points = spawner::get_spawn_points_for_room();
    if points.is_empty() {
        points.push(Vec2::ZERO);
    }

    let elite_type = spawner::pick_enemy_type(rng, &pool);
    spawn_enemy(
        commands,
        assets,
        data,
        elite_type,
        points[0],
        floor_number.max(3),
        floor_multiplier,
        1.0,
        true,
    );

    for point in points.into_iter().skip(1).take(4) {
        let enemy_type = spawner::pick_enemy_type(rng, &pool);
        spawn_enemy(
            commands,
            assets,
            data,
            enemy_type,
            point,
            floor_number,
            floor_multiplier,
            1.0,
            false,
        );
    }
}

fn pick_weighted_event(rng: &mut GameRng) -> EventType {
    let weighted = [
        (EventType::BulletMaze, 1_u32),
        (EventType::MemoryBlocks, 1),
        (EventType::TimedCollect, 1),
        (EventType::Gambler, 2),
        (EventType::BloodPact, 2),
        (EventType::Treasure, 2),
        (EventType::HealingSpring, 2),
        (EventType::Merchant, 2),
        (EventType::GoldAltar, 1),
        (EventType::MysticBlacksmith, 1),
        (EventType::WheelOfFate, 1),
        (EventType::TravelerGift, 1),
        (EventType::SacrificeAltar, 1),
        (EventType::TimedChallenge, 1),
        (EventType::EliteEncounter, 1),
        (EventType::FlawlessTrial, 1),
        (EventType::CursedVault, 1),
    ];
    debug_assert_eq!(weighted.len(), EventType::PHASE3_EVENTS.len());
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
    EventType::BulletMaze
}

fn puzzle_config_for_event(
    data: Option<&GameDataRegistry>,
    event_type: EventType,
) -> Option<&PuzzleEventConfig> {
    data?
        .events
        .events
        .iter()
        .find(|event| event.id == event_type.config_id())
        .and_then(|event| event.puzzle.as_ref())
}

#[derive(Debug, Clone, Copy)]
enum AugmentPool {
    Any,
    CommonOnly,
    EliteOnly,
    LegendaryOnly,
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
            AugmentPool::LegendaryOnly => augment.rarity == AugmentRarity::Legendary,
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
                augment.description_for_stacks(1).to_string(),
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

#[derive(Debug, Clone)]
struct ForgeOffer {
    sacrifice_id: AugmentId,
    sacrifice_title: String,
    reward_id: AugmentId,
    reward_title: String,
    reward_description: String,
}

fn build_forge_offers(
    data: &GameDataRegistry,
    rng: &mut GameRng,
    inventory: Option<&AugmentInventory>,
    count: usize,
) -> Vec<ForgeOffer> {
    let Some(inventory) = inventory else {
        return Vec::new();
    };

    let mut held = inventory.augments.iter().collect::<Vec<_>>();
    rng.shuffle(&mut held);
    held.into_iter()
        .filter_map(|held| {
            let sacrifice = augment_definition(data, held.id)?;
            let reward = pick_higher_rarity_same_category(data, rng, sacrifice)?;
            Some(ForgeOffer {
                sacrifice_id: sacrifice.id,
                sacrifice_title: sacrifice.title.clone(),
                reward_id: reward.id,
                reward_title: reward.title.clone(),
                reward_description: reward.description_for_stacks(1).to_string(),
            })
        })
        .take(count)
        .collect()
}

fn pick_higher_rarity_same_category<'a>(
    data: &'a GameDataRegistry,
    rng: &mut GameRng,
    sacrifice: &AugmentConfig,
) -> Option<&'a AugmentConfig> {
    let target_rarity = next_rarity(sacrifice.rarity)?;
    let mut candidates = data
        .augments
        .augments
        .iter()
        .filter(|augment| {
            augment.category == sacrifice.category
                && augment.rarity == target_rarity
                && augment.id != sacrifice.id
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        candidates = data
            .augments
            .augments
            .iter()
            .filter(|augment| {
                augment.category == sacrifice.category
                    && rarity_rank(augment.rarity) > rarity_rank(sacrifice.rarity)
                    && augment.id != sacrifice.id
            })
            .collect::<Vec<_>>();
    }
    rng.shuffle(&mut candidates);
    candidates.into_iter().next()
}

fn augment_definition(data: &GameDataRegistry, id: AugmentId) -> Option<&AugmentConfig> {
    data.augments
        .augments
        .iter()
        .find(|augment| augment.id == id)
}

fn next_rarity(rarity: AugmentRarity) -> Option<AugmentRarity> {
    match rarity {
        AugmentRarity::Common => Some(AugmentRarity::Elite),
        AugmentRarity::Elite => Some(AugmentRarity::Legendary),
        AugmentRarity::Legendary => None,
    }
}

fn rarity_rank(rarity: AugmentRarity) -> u8 {
    match rarity {
        AugmentRarity::Common => 0,
        AugmentRarity::Elite => 1,
        AugmentRarity::Legendary => 2,
    }
}

fn traveler_intel_lines(layout: &FloorLayout, current_room: RoomId) -> Vec<String> {
    let mut upcoming = layout
        .rooms
        .iter()
        .filter(|room| room.id.0 > current_room.0)
        .map(|room| room_type_label(room.room_type).to_string())
        .take(3)
        .collect::<Vec<_>>();
    if upcoming.is_empty() {
        upcoming.push("暂无可读路线情报".to_string());
    }
    vec![format!("后续房间：{}", upcoming.join(" / "))]
}

fn room_type_label(room_type: RoomType) -> &'static str {
    match room_type {
        RoomType::Start => "起点",
        RoomType::Normal => "战斗",
        RoomType::Shop => "商店",
        RoomType::Reward => "奖励",
        RoomType::Event => "事件",
        RoomType::Elite => "精英",
        RoomType::Boss => "Boss",
    }
}

fn trade_gold_for_augment(data: Option<&GameDataRegistry>, id: AugmentId) -> u32 {
    data.and_then(|registry| augment_definition(registry, id))
        .map(|augment| match augment.rarity {
            AugmentRarity::Common => 40,
            AugmentRarity::Elite => 80,
            AugmentRarity::Legendary => 140,
        })
        .unwrap_or(40)
}

fn leave_choice() -> EventChoice {
    EventChoice {
        label: "离开".to_string(),
        description: "放弃这次事件，立即返回地图。".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventInputOutcome {
    StayOpen,
    Leave,
    Complete,
}

struct EventApplyResult {
    outcome: EventInputOutcome,
    feedback: Option<UiFeedbackEvent>,
}

fn apply_choice_payload(
    payload: EventChoicePayload,
    gold: &mut Gold,
    health: &mut Health,
    energy: &mut Energy,
    inventory: &mut AugmentInventory,
    level: &mut PlayerLevel,
    data: Option<&GameDataRegistry>,
    rng: &mut GameRng,
) -> EventApplyResult {
    match payload {
        EventChoicePayload::Leave => EventApplyResult {
            outcome: EventInputOutcome::Leave,
            feedback: None,
        },
        EventChoicePayload::Unavailable { title, message } => EventApplyResult {
            outcome: EventInputOutcome::StayOpen,
            feedback: Some(UiFeedbackEvent {
                title,
                lines: vec![message],
                severity: UiFeedbackSeverity::Warning,
                requires_ack: false,
                return_phase: GamePhase::EventRoom,
            }),
        },
        EventChoicePayload::Gambler { cost, augment_id } => {
            if gold.0 < cost {
                warn!("金币不足：需要 {}，当前 {}", cost, gold.0);
                return EventApplyResult {
                    outcome: EventInputOutcome::StayOpen,
                    feedback: Some(insufficient_gold_feedback(cost, gold.0)),
                };
            }
            gold.0 -= cost;
            let grant = inventory.grant(augment_id);
            EventApplyResult {
                outcome: EventInputOutcome::Complete,
                feedback: Some(success_feedback(
                    "赌徒结算",
                    event_lines(vec![format!("-{} 金币", cost)], grant, data),
                )),
            }
        }
        EventChoicePayload::BloodPact {
            hp_fraction,
            augment_id,
        } => {
            let before = health.current;
            health.current = (health.current * (1.0 - hp_fraction)).clamp(1.0, health.max);
            let grant = inventory.grant(augment_id);
            EventApplyResult {
                outcome: EventInputOutcome::Complete,
                feedback: Some(success_feedback(
                    "血契结算",
                    event_lines(
                        vec![format!("生命: {:.0} -> {:.0}", before, health.current)],
                        grant,
                        data,
                    ),
                )),
            }
        }
        EventChoicePayload::Treasure {
            augment_id,
            gold_bonus,
        } => {
            let grant = inventory.grant(augment_id);
            gold.0 = gold.0.saturating_add(gold_bonus);
            EventApplyResult {
                outcome: EventInputOutcome::Complete,
                feedback: Some(success_feedback(
                    "宝藏结算",
                    event_lines(vec![format!("+{} 金币", gold_bonus)], grant, data),
                )),
            }
        }
        EventChoicePayload::HealingSpring {
            hp_fraction,
            energy_flat,
            energy_fraction,
        } => {
            let before_hp = health.current;
            let before_energy = energy.current;
            if hp_fraction > 0.0 {
                health.current = (health.current + health.max * hp_fraction).min(health.max);
            }
            if energy_flat > 0.0 || energy_fraction > 0.0 {
                let gain = energy_flat + energy.max * energy_fraction;
                energy.current = (energy.current + gain).min(energy.max);
            }
            let mut lines = Vec::new();
            if (health.current - before_hp).abs() > f32::EPSILON {
                lines.push(format!("生命: {:.0} -> {:.0}", before_hp, health.current));
            }
            if (energy.current - before_energy).abs() > f32::EPSILON {
                lines.push(format!(
                    "能量: {:.0} -> {:.0}",
                    before_energy, energy.current
                ));
            }
            if lines.is_empty() {
                lines.push("状态已满，没有额外恢复。".to_string());
            }
            EventApplyResult {
                outcome: EventInputOutcome::Complete,
                feedback: Some(success_feedback("治愈泉", lines)),
            }
        }
        EventChoicePayload::Merchant { cost, augment_id } => {
            if gold.0 < cost {
                warn!("金币不足：需要 {}，当前 {}", cost, gold.0);
                return EventApplyResult {
                    outcome: EventInputOutcome::StayOpen,
                    feedback: Some(insufficient_gold_feedback(cost, gold.0)),
                };
            }
            gold.0 -= cost;
            let grant = inventory.grant(augment_id);
            EventApplyResult {
                outcome: EventInputOutcome::Complete,
                feedback: Some(success_feedback(
                    "购买成功",
                    event_lines(vec![format!("-{} 金币", cost)], grant, data),
                )),
            }
        }
        EventChoicePayload::GoldAltar {
            gold: gain,
            hp_fraction,
            energy_cost,
        } => {
            if energy_cost > 0.0 && energy.current + f32::EPSILON < energy_cost {
                return EventApplyResult {
                    outcome: EventInputOutcome::StayOpen,
                    feedback: Some(insufficient_energy_feedback(energy_cost, energy.current)),
                };
            }
            let before = health.current;
            let before_energy = energy.current;
            if hp_fraction > 0.0 {
                health.current = (health.current * (1.0 - hp_fraction)).clamp(1.0, health.max);
            }
            if energy_cost > 0.0 {
                energy.current = (energy.current - energy_cost).max(0.0);
            }
            gold.0 = gold.0.saturating_add(gain);
            let mut lines = Vec::new();
            if hp_fraction > 0.0 {
                lines.push(format!("生命: {:.0} -> {:.0}", before, health.current));
            }
            if energy_cost > 0.0 {
                lines.push(format!(
                    "能量: {:.0} -> {:.0}",
                    before_energy, energy.current
                ));
            }
            lines.push(format!("+{} 金币", gain));
            EventApplyResult {
                outcome: EventInputOutcome::Complete,
                feedback: Some(success_feedback("金币祭坛", lines)),
            }
        }
        EventChoicePayload::MysticForge {
            sacrifice_id,
            reward_id,
        } => {
            let Some(removed) = inventory.remove(sacrifice_id) else {
                return EventApplyResult {
                    outcome: EventInputOutcome::StayOpen,
                    feedback: Some(UiFeedbackEvent {
                        title: "神秘铸炉".to_string(),
                        lines: vec!["要献祭的强化已经不存在。".to_string()],
                        severity: UiFeedbackSeverity::Warning,
                        requires_ack: false,
                        return_phase: GamePhase::EventRoom,
                    }),
                };
            };
            let grant = inventory.grant(reward_id);
            EventApplyResult {
                outcome: EventInputOutcome::Complete,
                feedback: Some(success_feedback(
                    "神秘铸炉",
                    event_lines(
                        vec![format!("献祭：{}", describe_held_augment(removed.id, data))],
                        grant,
                        data,
                    ),
                )),
            }
        }
        EventChoicePayload::WheelOfFate => {
            apply_wheel_of_fate(gold, health, inventory, level, data, rng)
        }
        EventChoicePayload::TravelerRest => {
            let before = health.current;
            health.current = (health.current + health.max * 0.20).min(health.max);
            EventApplyResult {
                outcome: EventInputOutcome::Complete,
                feedback: Some(success_feedback(
                    "旅者休憩",
                    vec![format!("生命: {:.0} -> {:.0}", before, health.current)],
                )),
            }
        }
        EventChoicePayload::TravelerIntel { lines } => EventApplyResult {
            outcome: EventInputOutcome::Complete,
            feedback: Some(UiFeedbackEvent::card(
                "旅者情报",
                lines,
                UiFeedbackSeverity::Info,
                GamePhase::Playing,
            )),
        },
        EventChoicePayload::TravelerTrade {
            sacrifice_id,
            gold: gain,
        } => {
            let Some(removed) = inventory.remove(sacrifice_id) else {
                return EventApplyResult {
                    outcome: EventInputOutcome::StayOpen,
                    feedback: Some(UiFeedbackEvent {
                        title: "旅者交易".to_string(),
                        lines: vec!["要交易的强化已经不存在。".to_string()],
                        severity: UiFeedbackSeverity::Warning,
                        requires_ack: false,
                        return_phase: GamePhase::EventRoom,
                    }),
                };
            };
            gold.0 = gold.0.saturating_add(gain);
            EventApplyResult {
                outcome: EventInputOutcome::Complete,
                feedback: Some(success_feedback(
                    "旅者交易",
                    vec![
                        format!("交出：{}", describe_held_augment(removed.id, data)),
                        format!("+{} 金币", gain),
                    ],
                )),
            }
        }
        EventChoicePayload::SacrificeAltar { augment_id } => {
            let before_max = health.max;
            let before_current = health.current;
            health.max = (health.max * 0.80).max(1.0);
            health.current = health.current.min(health.max).max(1.0);
            let grant = inventory.grant(augment_id);
            EventApplyResult {
                outcome: EventInputOutcome::Complete,
                feedback: Some(success_feedback(
                    "献祭祭坛",
                    event_lines(
                        vec![
                            format!("最大生命: {:.0} -> {:.0}", before_max, health.max),
                            format!("当前生命: {:.0} -> {:.0}", before_current, health.current),
                        ],
                        grant,
                        data,
                    ),
                )),
            }
        }
    }
}

fn success_feedback(title: impl Into<String>, lines: Vec<String>) -> UiFeedbackEvent {
    UiFeedbackEvent::card(
        title,
        lines,
        UiFeedbackSeverity::Success,
        GamePhase::Playing,
    )
}

fn insufficient_gold_feedback(cost: u32, current: u32) -> UiFeedbackEvent {
    UiFeedbackEvent {
        title: "金币不足".to_string(),
        lines: vec![format!("需要 {} 金币，当前只有 {}。", cost, current)],
        severity: UiFeedbackSeverity::Warning,
        requires_ack: false,
        return_phase: GamePhase::EventRoom,
    }
}

fn insufficient_energy_feedback(cost: f32, current: f32) -> UiFeedbackEvent {
    UiFeedbackEvent {
        title: "能量不足".to_string(),
        lines: vec![format!("需要 {:.0} 能量，当前只有 {:.0}。", cost, current)],
        severity: UiFeedbackSeverity::Warning,
        requires_ack: false,
        return_phase: GamePhase::EventRoom,
    }
}

fn apply_wheel_of_fate(
    gold: &mut Gold,
    health: &mut Health,
    inventory: &mut AugmentInventory,
    level: &mut PlayerLevel,
    data: Option<&GameDataRegistry>,
    rng: &mut GameRng,
) -> EventApplyResult {
    let good = rng.gen_bool(0.5);
    let roll = rng.gen_range_f32(0.0, 3.0).floor() as u32;
    let mut lines = Vec::new();

    if good {
        match roll {
            0 => {
                gold.0 = gold.0.saturating_add(30);
                lines.push("+30 金币".to_string());
            }
            1 => {
                level.level = level.level.saturating_add(1);
                level.xp_to_next = PlayerLevel::xp_threshold(level.level);
                lines.push(format!("等级提升到 {}", level.level));
            }
            _ => {
                if let Some(augment_id) = pick_random_augment_id(data, rng, AugmentPool::Any) {
                    let grant = inventory.grant(augment_id);
                    lines.extend(describe_augment_grant(grant, data));
                } else {
                    gold.0 = gold.0.saturating_add(30);
                    lines.push("+30 金币".to_string());
                }
            }
        }
    } else {
        match roll {
            0 => {
                let before = gold.0;
                gold.0 = gold.0.saturating_sub(20);
                lines.push(format!("金币: {} -> {}", before, gold.0));
            }
            1 => {
                let before = health.current;
                health.current = (health.current * 0.80).clamp(1.0, health.max);
                lines.push(format!("生命: {:.0} -> {:.0}", before, health.current));
            }
            _ => {
                if inventory.augments.is_empty() {
                    let before = gold.0;
                    gold.0 = gold.0.saturating_sub(20);
                    lines.push(format!("没有强化可失去，金币: {} -> {}", before, gold.0));
                } else {
                    let index = rng
                        .gen_range_f32(0.0, inventory.augments.len() as f32)
                        .floor() as usize;
                    if let Some(removed) =
                        inventory.remove_at(index.min(inventory.augments.len() - 1))
                    {
                        lines.push(format!(
                            "失去强化：{}",
                            describe_held_augment(removed.id, data)
                        ));
                    }
                }
            }
        }
    }

    EventApplyResult {
        outcome: EventInputOutcome::Complete,
        feedback: Some(UiFeedbackEvent::card(
            if good {
                "命运之轮：好运"
            } else {
                "命运之轮：代价"
            },
            lines,
            if good {
                UiFeedbackSeverity::Success
            } else {
                UiFeedbackSeverity::Warning
            },
            GamePhase::Playing,
        )),
    }
}

fn event_lines(
    mut prefix: Vec<String>,
    grant: AugmentGrantResult,
    data: Option<&GameDataRegistry>,
) -> Vec<String> {
    prefix.extend(describe_augment_grant(grant, data));
    prefix
}

fn describe_held_augment(id: AugmentId, data: Option<&GameDataRegistry>) -> String {
    data.and_then(|registry| augment_definition(registry, id))
        .map(|augment| {
            format!(
                "{} · {}",
                augment.title,
                crate::ui::widgets::rarity_label(augment.rarity)
            )
        })
        .unwrap_or_else(|| format!("{id:?}"))
}

fn describe_augment_grant(
    grant: AugmentGrantResult,
    data: Option<&GameDataRegistry>,
) -> Vec<String> {
    let (title, rarity, effect) = data
        .and_then(|registry| {
            registry
                .augments
                .augments
                .iter()
                .find(|augment| augment.id == grant.id)
                .map(|augment| {
                    (
                        augment.title.as_str(),
                        augment.rarity,
                        augment.description_for_stacks(grant.after_stacks),
                    )
                })
        })
        .unwrap_or(("未知强化", AugmentRarity::Common, "效果未配置"));

    let change = if grant.before_stacks == 0 {
        format!(
            "获得强化：{} · {} Lv{}",
            title,
            crate::ui::widgets::rarity_label(rarity),
            grant.after_stacks
        )
    } else if grant.after_stacks > grant.before_stacks {
        format!(
            "强化升级：{} Lv{} -> Lv{}",
            title, grant.before_stacks, grant.after_stacks
        )
    } else {
        format!("强化已达上限：{} Lv{}", title, grant.after_stacks)
    };

    vec![change, format!("效果：{effect}")]
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

    fn test_layout() -> FloorLayout {
        FloorLayout {
            current: RoomId(1),
            rooms: vec![
                crate::gameplay::map::room::RoomData {
                    id: RoomId(1),
                    room_type: RoomType::Event,
                    mystery: false,
                    connections: crate::gameplay::map::room::RoomConnections { exits: vec![] },
                    bounds: crate::gameplay::map::room::RoomBounds {
                        half_size: Vec2::splat(300.0),
                    },
                },
                crate::gameplay::map::room::RoomData {
                    id: RoomId(2),
                    room_type: RoomType::Normal,
                    mystery: false,
                    connections: crate::gameplay::map::room::RoomConnections { exits: vec![] },
                    bounds: crate::gameplay::map::room::RoomBounds {
                        half_size: Vec2::splat(300.0),
                    },
                },
                crate::gameplay::map::room::RoomData {
                    id: RoomId(3),
                    room_type: RoomType::Reward,
                    mystery: false,
                    connections: crate::gameplay::map::room::RoomConnections { exits: vec![] },
                    bounds: crate::gameplay::map::room::RoomBounds {
                        half_size: Vec2::splat(300.0),
                    },
                },
                crate::gameplay::map::room::RoomData {
                    id: RoomId(4),
                    room_type: RoomType::Boss,
                    mystery: false,
                    connections: crate::gameplay::map::room::RoomConnections { exits: vec![] },
                    bounds: crate::gameplay::map::room::RoomBounds {
                        half_size: Vec2::splat(300.0),
                    },
                },
            ],
        }
    }

    #[test]
    fn event_type_matrix_has_three_puzzles_ten_noncombat_four_combat() {
        assert_eq!(EventType::PHASE3_EVENTS.len(), 17);
        let puzzles = EventType::PHASE3_EVENTS
            .iter()
            .filter(|event| event.is_puzzle())
            .count();
        let combat = EventType::PHASE3_EVENTS
            .iter()
            .filter(|event| event.is_combat())
            .count();
        let noncombat = EventType::PHASE3_EVENTS.len() - puzzles - combat;

        assert_eq!(puzzles, 3);
        assert_eq!(noncombat, 10);
        assert_eq!(combat, 4);
        for event in EventType::PHASE3_EVENTS {
            assert!(!event.title().is_empty());
            assert!(!event.description().is_empty());
            assert!(!event.symbol().is_empty());
        }
    }

    #[test]
    fn every_noncombat_event_builds_expected_choice_surface() {
        let registry = load_test_registry();
        let layout = test_layout();
        let mut rng = GameRng::default();
        rng.reseed(11);
        let inventory = AugmentInventory::default();

        for event in EventType::PHASE3_EVENTS
            .into_iter()
            .filter(|event| !event.is_puzzle() && !event.is_combat())
        {
            let mut active = ActiveEvent {
                event_type: Some(event),
                ..Default::default()
            };
            configure_non_combat_event(
                &mut active,
                Some(&registry),
                &mut rng,
                Some(&inventory),
                &layout,
                RoomId(1),
            );

            assert!(
                !active.choices.is_empty(),
                "{event:?} should offer at least one action"
            );
            assert_eq!(active.choices.len(), active.choice_payloads.len());
            let expected_choices = match event {
                EventType::Gambler => 3,
                EventType::BloodPact => 3,
                EventType::Treasure => 1,
                EventType::HealingSpring => 3,
                EventType::Merchant => 3,
                EventType::GoldAltar => 4,
                EventType::MysticBlacksmith => 2,
                EventType::WheelOfFate => 2,
                EventType::TravelerGift => 3,
                EventType::SacrificeAltar => 2,
                _ => unreachable!(),
            };
            assert_eq!(active.choices.len(), expected_choices, "{event:?}");
        }
    }

    #[test]
    fn gambler_success_reports_cost_and_granted_augment() {
        let registry = load_test_registry();
        let mut gold = Gold(100);
        let mut health = Health {
            current: 80.0,
            max: 100.0,
        };
        let mut energy = Energy {
            current: 20.0,
            max: 100.0,
        };
        let mut inventory = AugmentInventory::default();
        let mut level = PlayerLevel::default();
        let mut rng = GameRng::default();

        let result = apply_choice_payload(
            EventChoicePayload::Gambler {
                cost: 50,
                augment_id: AugmentId::Piercing,
            },
            &mut gold,
            &mut health,
            &mut energy,
            &mut inventory,
            &mut level,
            Some(&registry),
            &mut rng,
        );

        assert_eq!(result.outcome, EventInputOutcome::Complete);
        assert_eq!(gold.0, 50);
        assert_eq!(inventory.stacks(AugmentId::Piercing), 1);
        let feedback = result.feedback.expect("gambler should show feedback");
        assert!(feedback.requires_ack);
        assert_eq!(feedback.title, "赌徒结算");
        assert!(feedback.lines.iter().any(|line| line.contains("-50 金币")));
        assert!(feedback.lines.iter().any(|line| line.contains("获得强化")));
    }

    #[test]
    fn gambler_insufficient_gold_stays_open_without_charging() {
        let mut gold = Gold(20);
        let mut health = Health {
            current: 80.0,
            max: 100.0,
        };
        let mut energy = Energy {
            current: 20.0,
            max: 100.0,
        };
        let mut inventory = AugmentInventory::default();
        let mut level = PlayerLevel::default();
        let mut rng = GameRng::default();

        let result = apply_choice_payload(
            EventChoicePayload::Gambler {
                cost: 50,
                augment_id: AugmentId::Piercing,
            },
            &mut gold,
            &mut health,
            &mut energy,
            &mut inventory,
            &mut level,
            None,
            &mut rng,
        );

        assert_eq!(result.outcome, EventInputOutcome::StayOpen);
        assert_eq!(gold.0, 20);
        assert_eq!(inventory.stacks(AugmentId::Piercing), 0);
        let feedback = result.feedback.expect("failure should show feedback");
        assert!(!feedback.requires_ack);
        assert_eq!(feedback.title, "金币不足");
        assert_eq!(feedback.return_phase, GamePhase::EventRoom);
    }
}
