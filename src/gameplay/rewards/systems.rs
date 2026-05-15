use bevy::prelude::*;
use bevy::utils::HashSet;

use crate::core::assets::GameAssets;
use crate::core::events::{RoomClearedEvent, SpawnEnemyEvent};
use crate::data::definitions::{AugmentConfig, RewardScalingConfig};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentInventory, AugmentRarity};
use crate::gameplay::enemy::systems::{ClearGrace, EnemySpawnCount, SpawnedForRoom};
use crate::gameplay::map::generator::{build_rooms, reset_player_for_floor, spawn_current_room};
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId, RoomType};
use crate::gameplay::map::transitions::RoomTransition;
use crate::gameplay::map::{InGameEntity, VisitedRooms};
use crate::gameplay::player::components::{
    DashState, Energy, Health, Player, RewardModifiers, Velocity,
};
use crate::gameplay::progression::experience::{PlayerLevel, build_levelup_options};
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::session_core::{
    PostRewardDecision, SessionMode, SessionRuleContext, on_room_cleared,
};
use crate::states::{AppState, GamePhase, RoomState};
use crate::ui::augment_select::{AugmentChoiceOption, AugmentChoices};
use crate::ui::levelup_select::LevelUpChoices;
use crate::utils::entity::safe_despawn_recursive;
use crate::utils::rng::GameRng;

#[derive(Resource, Debug, Default, Clone)]
pub struct RewardRoomClaims {
    pub rooms: HashSet<RoomId>,
}

#[derive(Debug, Clone)]
pub enum RewardRoomAugmentService {
    Upgrade(Vec<AugmentChoiceOption>),
    Awakening(Vec<AugmentChoiceOption>),
}

#[derive(Debug, Clone)]
pub struct SanctuaryDraft {
    pub augment_service: RewardRoomAugmentService,
}

#[derive(Debug, Clone, Default)]
pub enum RewardFlowStep {
    #[default]
    Inactive,
    Sanctuary(SanctuaryDraft),
    UpgradePick(Vec<AugmentChoiceOption>),
    AwakeningPick(Vec<AugmentChoiceOption>),
}

#[derive(Resource, Debug, Default, Clone)]
pub struct RewardFlow {
    pub room: Option<RoomId>,
    pub step: RewardFlowStep,
    pub spawn_portal: bool,
    pub portal_is_victory: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardUiAction {
    Select(usize),
    Back,
}

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RewardPendingAction(pub Option<RewardUiAction>);

#[derive(Component)]
pub struct BossPortal {
    pub is_victory: bool,
}

pub struct RewardsSystemsPlugin;

impl Plugin for RewardsSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RewardRoomClaims>()
            .init_resource::<RewardPendingAction>()
            .init_resource::<RewardFlow>()
            .init_resource::<GameRng>()
            .add_systems(
                Update,
                offer_reward_in_reward_room
                    .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
            )
            .add_systems(
                Update,
                enter_reward_selection
                    .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
            )
            .add_systems(
                Update,
                (spawn_boss_portal, boss_portal_interact)
                    .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
            )
            .add_systems(
                OnEnter(GamePhase::RewardSelect),
                crate::ui::reward_select::setup_reward_ui,
            )
            .add_systems(
                Update,
                (
                    handle_reward_choice_input,
                    crate::ui::reward_select::reward_ui_input_system,
                    crate::ui::reward_select::update_reward_ui,
                )
                    .run_if(in_state(GamePhase::RewardSelect)),
            )
            .add_systems(
                OnExit(GamePhase::RewardSelect),
                crate::ui::reward_select::cleanup_reward_ui,
            )
            .add_systems(
                Update,
                apply_reward_choice
                    .run_if(in_state(GamePhase::RewardSelect))
                    .after(handle_reward_choice_input)
                    .after(crate::ui::reward_select::reward_ui_input_system),
            );
    }
}

fn offer_reward_in_reward_room(
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    data: Option<Res<GameDataRegistry>>,
    transition: Option<Res<RoomTransition>>,
    mut claims: ResMut<RewardRoomClaims>,
    mut flow: ResMut<RewardFlow>,
    mut pending_action: ResMut<RewardPendingAction>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut room_state: ResMut<RoomState>,
    mut rng: ResMut<GameRng>,
    player_q: Query<Option<&AugmentInventory>, With<Player>>,
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
    if layout.is_changed() {
        claims.rooms.clear();
    }

    let Some(room) = layout.room(current.0) else {
        return;
    };
    let reward_room_enabled = data
        .as_deref()
        .map(|registry| registry.balance.reward_rooms_give_choice)
        .unwrap_or(true);
    if !reward_room_enabled || room.room_type != RoomType::Reward {
        return;
    }
    if !claims.rooms.insert(current.0) {
        return;
    }

    let inventory = player_q.get_single().ok().flatten();
    let draft = data
        .as_deref()
        .map(|registry| build_sanctuary_draft(registry.augments.augments.as_slice(), &mut rng, inventory))
        .unwrap_or_else(|| SanctuaryDraft {
            augment_service: RewardRoomAugmentService::Awakening(Vec::new()),
        });

    *room_state = RoomState::Locked;
    flow.room = Some(current.0);
    flow.step = RewardFlowStep::Sanctuary(draft);
    pending_action.0 = None;
    next_state.set(GamePhase::RewardSelect);
}

fn enter_reward_selection(
    mut room_cleared: EventReader<RoomClearedEvent>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut flow: ResMut<RewardFlow>,
    mut pending_action: ResMut<RewardPendingAction>,
    mut rng: ResMut<GameRng>,
    mut augment_choices: ResMut<AugmentChoices>,
    data: Option<Res<GameDataRegistry>>,
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    floor: Option<Res<FloorNumber>>,
    mut player_q: Query<(&RewardModifiers, &mut Health, Option<&AugmentInventory>), With<Player>>,
) {
    let Some(ev) = room_cleared.read().next() else {
        return;
    };

    pending_action.0 = None;

    let (Some(layout), Some(current)) = (layout.as_deref(), current.as_deref()) else {
        return;
    };
    if ev.room != current.0 {
        return;
    }

    let Some(room) = layout.room(current.0) else {
        return;
    };
    if matches!(room.room_type, RoomType::Reward | RoomType::Event) {
        return;
    }

    let decision = on_room_cleared(SessionRuleContext {
        mode: SessionMode::Solo,
        floor: floor.as_deref().map(|value| value.0).unwrap_or(1),
        total_floors: data
            .as_deref()
            .map(|registry| registry.balance.total_floors.max(1))
            .unwrap_or(4),
        boss_gives_victory: data
            .as_deref()
            .map(|registry| registry.balance.boss_room_gives_victory)
            .unwrap_or(false),
        room_type: room.room_type,
    });

    if decision.heal_alive_fraction > 0.0 {
        if let Ok((_, mut health, _)) = player_q.get_single_mut() {
            let heal = health.max * decision.heal_alive_fraction;
            health.current = (health.current + heal).min(health.max);
        }
    }

    let is_boss = room.room_type == RoomType::Boss;
    if !is_boss {
        let is_elite_room = room.room_type == RoomType::Elite;
        let should_offer_augment = is_elite_room || rng.gen_bool(0.40);
        if should_offer_augment {
            if let Some(registry) = data.as_deref() {
                let inventory = player_q.get_single().ok().and_then(|(_, _, inv)| inv);
                let generated = generate_augment_choices(
                    registry.augments.augments.as_slice(),
                    &mut rng,
                    false,
                    inventory,
                );
                if !generated.is_empty() {
                    augment_choices.options = generated;
                    augment_choices.return_state = Some(GamePhase::Playing);
                    next_state.set(GamePhase::AugmentSelect);
                }
            }
        }
        return;
    }

    flow.spawn_portal = true;
    flow.portal_is_victory = decision.post_reward == PostRewardDecision::Victory;

    if let Some(registry) = data.as_deref() {
        let inventory = player_q.get_single().ok().and_then(|(_, _, inv)| inv);
        let generated = generate_augment_choices(
            registry.augments.augments.as_slice(),
            &mut rng,
            true,
            inventory,
        );
        if !generated.is_empty() {
            augment_choices.options = generated;
            augment_choices.return_state = Some(GamePhase::Playing);
            next_state.set(GamePhase::AugmentSelect);
            return;
        }
    }

    next_state.set(GamePhase::Playing);
}

fn handle_reward_choice_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    flow: Res<RewardFlow>,
    mut pending_action: ResMut<RewardPendingAction>,
) {
    match &flow.step {
        RewardFlowStep::Inactive => {}
        RewardFlowStep::Sanctuary(_) => {
            if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1) {
                pending_action.0 = Some(RewardUiAction::Select(0));
            } else if keyboard.just_pressed(KeyCode::Digit2)
                || keyboard.just_pressed(KeyCode::Numpad2)
            {
                pending_action.0 = Some(RewardUiAction::Select(1));
            } else if keyboard.just_pressed(KeyCode::Digit3)
                || keyboard.just_pressed(KeyCode::Numpad3)
            {
                pending_action.0 = Some(RewardUiAction::Select(2));
            }
        }
        RewardFlowStep::UpgradePick(options) | RewardFlowStep::AwakeningPick(options) => {
            if keyboard.just_pressed(KeyCode::Escape) {
                pending_action.0 = Some(RewardUiAction::Back);
                return;
            }
            if !options.is_empty()
                && (keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1))
            {
                pending_action.0 = Some(RewardUiAction::Select(0));
            } else if options.len() >= 2
                && (keyboard.just_pressed(KeyCode::Digit2)
                    || keyboard.just_pressed(KeyCode::Numpad2))
            {
                pending_action.0 = Some(RewardUiAction::Select(1));
            } else if options.len() >= 3
                && (keyboard.just_pressed(KeyCode::Digit3)
                    || keyboard.just_pressed(KeyCode::Numpad3))
            {
                pending_action.0 = Some(RewardUiAction::Select(2));
            }
        }
    }
}

fn apply_reward_choice(
    mut next_state: ResMut<NextState<GamePhase>>,
    mut flow: ResMut<RewardFlow>,
    mut pending_action: ResMut<RewardPendingAction>,
    mut room_state: ResMut<RoomState>,
    mut cleared: EventWriter<RoomClearedEvent>,
    mut levelup_choices: ResMut<LevelUpChoices>,
    floor: Option<Res<FloorNumber>>,
    data: Option<Res<GameDataRegistry>>,
    mut rng: ResMut<GameRng>,
    mut player_q: Query<(&mut Health, &mut Energy, &mut AugmentInventory, &mut PlayerLevel), With<Player>>,
) {
    let Some(action) = pending_action.0.take() else {
        return;
    };

    let step = flow.step.clone();
    match (step, action) {
        (RewardFlowStep::Inactive, _) => {}
        (RewardFlowStep::Sanctuary(_), RewardUiAction::Back) => {}
        (RewardFlowStep::Sanctuary(sanctuary), RewardUiAction::Select(index)) => match index {
            0 => {
                if let Ok((mut health, mut energy, _, _)) = player_q.get_single_mut() {
                    full_restore(&mut health, &mut energy);
                }
                finish_reward_room(
                    &mut flow,
                    &mut room_state,
                    &mut cleared,
                    &mut next_state,
                    GamePhase::Playing,
                );
            }
            1 => {
                let (options, is_upgrade) = match sanctuary.augment_service {
                    RewardRoomAugmentService::Upgrade(options) => (options, true),
                    RewardRoomAugmentService::Awakening(options) => (options, false),
                };
                if options.is_empty() {
                    // Degenerate state (augments config empty/broken, or every
                    // elite/legendary already owned): never leave the forge
                    // option as a silent dead button. Resolve the room so the
                    // run continues. Phase 3 (§4.6) replaces this with a
                    // guaranteed legendary grant.
                    warn!("圣所强化锻造无可用选项，安全收敛奖励房");
                    finish_reward_room(
                        &mut flow,
                        &mut room_state,
                        &mut cleared,
                        &mut next_state,
                        GamePhase::Playing,
                    );
                } else if is_upgrade {
                    flow.step = RewardFlowStep::UpgradePick(options);
                } else {
                    flow.step = RewardFlowStep::AwakeningPick(options);
                }
            }
            2 => {
                let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
                let default_scaling;
                let scaling = if let Some(data) = data.as_ref() {
                    &data.rewards.scaling
                } else {
                    default_scaling = RewardScalingConfig::default_config();
                    &default_scaling
                };

                if let Ok((health, _energy, _inventory, mut level)) = player_q.get_single_mut() {
                    level.level += 1;
                    level.xp_to_next = PlayerLevel::xp_threshold(level.level);
                    configure_revelation_choices(
                        &mut levelup_choices,
                        &mut rng,
                        scaling,
                        health.max,
                        floor_number,
                        level.level,
                    );
                    finish_reward_room(
                        &mut flow,
                        &mut room_state,
                        &mut cleared,
                        &mut next_state,
                        GamePhase::LevelUpSelect,
                    );
                }
            }
            _ => {}
        },
        (RewardFlowStep::UpgradePick(options), RewardUiAction::Back) => {
            // Return to the sanctuary preserving the SAME options. Never
            // rebuild via RNG here, otherwise Esc-then-reselect would let the
            // player re-roll the forge/awakening offers for free.
            flow.step = RewardFlowStep::Sanctuary(SanctuaryDraft {
                augment_service: RewardRoomAugmentService::Upgrade(options),
            });
        }
        (RewardFlowStep::AwakeningPick(options), RewardUiAction::Back) => {
            flow.step = RewardFlowStep::Sanctuary(SanctuaryDraft {
                augment_service: RewardRoomAugmentService::Awakening(options),
            });
        }
        (RewardFlowStep::UpgradePick(options), RewardUiAction::Select(index))
        | (RewardFlowStep::AwakeningPick(options), RewardUiAction::Select(index)) => {
            let Some(choice) = options.get(index) else {
                return;
            };
            if let Ok((_, _, mut inventory, _)) = player_q.get_single_mut() {
                inventory.add(choice.id);
                finish_reward_room(
                    &mut flow,
                    &mut room_state,
                    &mut cleared,
                    &mut next_state,
                    GamePhase::Playing,
                );
            }
        }
    }
}

fn spawn_boss_portal(
    mut commands: Commands,
    mut flow: ResMut<RewardFlow>,
    assets: Res<GameAssets>,
    existing: Query<Entity, With<BossPortal>>,
) {
    if !flow.spawn_portal || !existing.is_empty() {
        return;
    }
    flow.spawn_portal = false;
    let is_victory = flow.portal_is_victory;

    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
            sprite: Sprite {
                color: Color::srgba(0.6, 0.2, 1.0, 0.9),
                custom_size: Some(Vec2::splat(40.0)),
                ..default()
            },
            ..default()
        },
        BossPortal { is_victory },
        InGameEntity,
        Name::new("BossPortal"),
    ));

    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                if is_victory {
                    "按 E 通关"
                } else {
                    "按 E 进入下一层"
                },
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 18.0,
                    color: Color::srgb(0.9, 0.8, 1.0),
                },
            )
            .with_justify(JustifyText::Center),
            transform: Transform::from_translation(Vec3::new(0.0, 30.0, 11.0)),
            ..default()
        },
        BossPortal { is_victory },
        InGameEntity,
        Name::new("BossPortalText"),
    ));
}

fn boss_portal_interact(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_q: ParamSet<(
        Query<&GlobalTransform, With<Player>>,
        Query<(&mut Transform, &mut Velocity, &mut DashState), With<Player>>,
    )>,
    portal_q: Query<(&GlobalTransform, &BossPortal), Without<Player>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GamePhase>>,
    ingame_entities: Query<(Entity, Option<&Player>), With<InGameEntity>>,
    mut floor: Option<ResMut<FloorNumber>>,
    mut spawned_for_room: ResMut<SpawnedForRoom>,
    mut grace: ResMut<ClearGrace>,
    mut spawn_count: ResMut<EnemySpawnCount>,
    mut rng: ResMut<GameRng>,
    data: Option<Res<GameDataRegistry>>,
    visited: Option<ResMut<VisitedRooms>>,
    spawn_events: EventWriter<SpawnEnemyEvent>,
) {
    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }

    let player_pos = {
        let player_positions = player_q.p0();
        let Ok(player_transform) = player_positions.get_single() else {
            return;
        };
        player_transform.translation().truncate()
    };

    let mut target_portal: Option<&BossPortal> = None;
    for (portal_transform, portal) in &portal_q {
        if player_pos.distance(portal_transform.translation().truncate()) <= 60.0 {
            target_portal = Some(portal);
            break;
        }
    }
    let Some(portal) = target_portal else {
        return;
    };

    if portal.is_victory {
        next_state.set(GamePhase::Victory);
        return;
    }

    for (entity, player) in &ingame_entities {
        if player.is_none() {
            safe_despawn_recursive(&mut commands, entity);
        }
    }

    if let Some(floor) = floor.as_mut() {
        floor.0 += 1;
    }
    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
    let layout = FloorLayout {
        rooms: build_rooms(data.as_deref(), floor_number, &mut rng),
        current: RoomId(0),
    };
    commands.insert_resource(RoomState::Idle);
    commands.insert_resource(RoomTransition::default());
    commands.insert_resource(CurrentRoom(layout.current));
    commands.insert_resource(layout);

    if let Some(mut visited) = visited {
        visited.0.clear();
        visited.0.insert(RoomId(0));
    }
    reset_player_for_floor(&mut player_q.p1());
    spawn_current_room(&mut commands, &spawn_events);

    spawned_for_room.0 = None;
    grace.last_room = None;
    grace.timer = Timer::from_seconds(0.0, TimerMode::Once);
    spawn_count.current = 0;

    next_state.set(GamePhase::Playing);
}

fn build_sanctuary_draft(
    augments: &[AugmentConfig],
    rng: &mut GameRng,
    inventory: Option<&AugmentInventory>,
) -> SanctuaryDraft {
    let upgrade_options = inventory
        .map(|inventory| build_upgrade_candidates(augments, inventory))
        .unwrap_or_default();
    let augment_service = if upgrade_options.is_empty() {
        RewardRoomAugmentService::Awakening(build_awakening_choices(augments, rng, inventory))
    } else {
        RewardRoomAugmentService::Upgrade(upgrade_options)
    };

    SanctuaryDraft { augment_service }
}

fn build_upgrade_candidates(
    augments: &[AugmentConfig],
    inventory: &AugmentInventory,
) -> Vec<AugmentChoiceOption> {
    inventory
        .augments
        .iter()
        .filter(|held| held.stacks == 1)
        .filter_map(|held| {
            augments
                .iter()
                .find(|augment| augment.id == held.id)
                .map(|augment| AugmentChoiceOption {
                    id: augment.id,
                    title: augment.title.clone(),
                    description: augment.upgraded_description.clone(),
                    rarity: augment.rarity,
                    is_upgrade: true,
                })
        })
        .take(3)
        .collect()
}

fn build_awakening_choices(
    augments: &[AugmentConfig],
    rng: &mut GameRng,
    inventory: Option<&AugmentInventory>,
) -> Vec<AugmentChoiceOption> {
    let owned = inventory
        .map(|inventory| {
            inventory
                .augments
                .iter()
                .map(|held| held.id)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    let mut pool = augments
        .iter()
        .filter(|augment| matches!(augment.rarity, AugmentRarity::Elite | AugmentRarity::Legendary))
        .filter(|augment| !owned.contains(&augment.id))
        .collect::<Vec<_>>();
    if pool.is_empty() {
        pool = augments
            .iter()
            .filter(|augment| matches!(augment.rarity, AugmentRarity::Elite | AugmentRarity::Legendary))
            .collect::<Vec<_>>();
    }
    rng.shuffle(&mut pool);
    pool.into_iter()
        .take(2)
        .map(|augment| AugmentChoiceOption {
            id: augment.id,
            title: augment.title.clone(),
            description: augment.description.clone(),
            rarity: augment.rarity,
            is_upgrade: false,
        })
        .collect()
}

fn generate_augment_choices(
    augments: &[AugmentConfig],
    rng: &mut GameRng,
    is_boss: bool,
    inventory: Option<&AugmentInventory>,
) -> Vec<AugmentChoiceOption> {
    let pool: Vec<_> = augments
        .iter()
        .filter(|augment| {
            if is_boss {
                matches!(augment.rarity, AugmentRarity::Elite | AugmentRarity::Legendary)
            } else {
                matches!(augment.rarity, AugmentRarity::Common | AugmentRarity::Elite)
            }
        })
        .collect();

    if pool.is_empty() {
        return Vec::new();
    }

    let mut indices: Vec<usize> = (0..pool.len()).collect();
    rng.shuffle(&mut indices);
    indices.truncate(3);

    indices
        .iter()
        .map(|&i| {
            let augment = pool[i];
            let is_upgrade = inventory.map(|inv| inv.has(augment.id)).unwrap_or(false);
            let description = if is_upgrade {
                &augment.upgraded_description
            } else {
                &augment.description
            };
            AugmentChoiceOption {
                id: augment.id,
                title: augment.title.clone(),
                description: description.clone(),
                rarity: augment.rarity,
                is_upgrade,
            }
        })
        .collect()
}

fn configure_revelation_choices(
    choices: &mut LevelUpChoices,
    rng: &mut GameRng,
    scaling: &RewardScalingConfig,
    max_health: f32,
    floor_number: u32,
    new_level: u32,
) {
    choices.options = build_levelup_options(rng, scaling, max_health, floor_number);
    choices.return_state = Some(GamePhase::Playing);
    choices.new_level = new_level;
}

fn full_restore(health: &mut Health, energy: &mut Energy) {
    health.current = health.max;
    energy.current = energy.max;
}

fn finish_reward_room(
    flow: &mut RewardFlow,
    room_state: &mut RoomState,
    cleared: &mut EventWriter<RoomClearedEvent>,
    next_state: &mut NextState<GamePhase>,
    target: GamePhase,
) {
    if let Some(room) = flow.room.take() {
        *room_state = RoomState::Cleared;
        cleared.send(RoomClearedEvent { room });
    }
    flow.step = RewardFlowStep::Inactive;
    next_state.set(target);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::definitions::AugmentConfig;
    use crate::gameplay::augment::data::{AugmentCategory, AugmentId, HeldAugment};

    fn seeded_rng(seed: u64) -> GameRng {
        let mut rng = GameRng::default();
        rng.reseed(seed);
        rng
    }

    fn sample_augment(
        id: AugmentId,
        rarity: AugmentRarity,
        title: &str,
        description: &str,
        upgraded_description: &str,
    ) -> AugmentConfig {
        AugmentConfig {
            id,
            category: AugmentCategory::General,
            rarity,
            title: title.to_string(),
            description: description.to_string(),
            upgraded_description: upgraded_description.to_string(),
            shop_cost: 0,
        }
    }

    fn sample_augments() -> Vec<AugmentConfig> {
        vec![
            sample_augment(
                AugmentId::GoldBonus,
                AugmentRarity::Common,
                "金币加成",
                "金币更多",
                "金币更多更多",
            ),
            sample_augment(
                AugmentId::Thorns,
                AugmentRarity::Elite,
                "荆棘",
                "受伤反伤",
                "反伤更强",
            ),
            sample_augment(
                AugmentId::Phoenix,
                AugmentRarity::Legendary,
                "不死鸟",
                "死亡复活",
                "复活更强",
            ),
        ]
    }

    #[test]
    fn heal_service_restores_health_and_energy() {
        let mut health = Health {
            current: 24.0,
            max: 100.0,
        };
        let mut energy = Energy {
            current: 10.0,
            max: 80.0,
        };

        full_restore(&mut health, &mut energy);

        assert_eq!(health.current, 100.0);
        assert_eq!(energy.current, 80.0);
    }

    #[test]
    fn upgrade_service_only_lists_single_stack_augments() {
        let mut inventory = AugmentInventory::default();
        inventory.augments = vec![
            HeldAugment {
                id: AugmentId::GoldBonus,
                stacks: 1,
            },
            HeldAugment {
                id: AugmentId::Thorns,
                stacks: 2,
            },
        ];

        let options = build_upgrade_candidates(&sample_augments(), &inventory);

        assert_eq!(options.len(), 1);
        assert_eq!(options[0].id, AugmentId::GoldBonus);
        assert!(options[0].is_upgrade);
    }

    #[test]
    fn sanctuary_falls_back_to_awakening_when_no_upgrade_exists() {
        let mut rng = seeded_rng(9);
        let inventory = AugmentInventory::default();

        let draft = build_sanctuary_draft(&sample_augments(), &mut rng, Some(&inventory));

        match draft.augment_service {
            RewardRoomAugmentService::Awakening(options) => {
                assert_eq!(options.len(), 2);
                assert!(options.iter().all(|option| {
                    matches!(option.rarity, AugmentRarity::Elite | AugmentRarity::Legendary)
                }));
            }
            RewardRoomAugmentService::Upgrade(_) => panic!("expected awakening fallback"),
        }
    }

    #[test]
    fn revelation_choices_return_to_ingame() {
        let mut rng = seeded_rng(11);
        let mut choices = LevelUpChoices::default();

        configure_revelation_choices(
            &mut choices,
            &mut rng,
            &RewardScalingConfig::default_config(),
            100.0,
            1,
            3,
        );

        assert_eq!(choices.return_state, Some(GamePhase::Playing));
        assert_eq!(choices.new_level, 3);
        assert_eq!(choices.options.len(), 4);
    }
}
