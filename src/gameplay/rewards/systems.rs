use bevy::prelude::*;
use bevy::utils::HashSet;

use crate::core::assets::GameAssets;
use crate::core::events::{RoomClearedEvent, SpawnEnemyEvent};
use crate::data::definitions::{AugmentConfig, RewardScalingConfig};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentInventory, AugmentRarity};
use crate::gameplay::augment::effects::{ArmorBroken, Frozen};
use crate::gameplay::enemy::systems::{ClearGrace, EnemySpawnCount, SpawnedForRoom};
use crate::gameplay::map::generator::{build_rooms, reset_player_for_floor, spawn_current_room};
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId, RoomType};
use crate::gameplay::map::transitions::RoomTransition;
use crate::gameplay::map::{InGameEntity, VisitedRooms};
use crate::gameplay::player::components::{
    DashState, Energy, Gold, Health, Player, RewardModifiers, Velocity,
};
use crate::gameplay::progression::experience::{PlayerLevel, build_levelup_options};
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::session_core::{
    PostRewardDecision, SessionMode, SessionRuleContext, on_room_cleared,
};
use crate::states::{AppState, GamePhase, RoomState};
use crate::ui::augment_select::{AugmentChoiceOption, AugmentChoices};
use crate::ui::feedback::{UiFeedbackEvent, UiFeedbackSeverity};
use crate::ui::levelup_select::LevelUpChoices;
use crate::ui::skill_select::{SkillChoiceOption, SkillChoices};
use crate::utils::entity::safe_despawn_recursive;
use crate::utils::rng::GameRng;

#[derive(Resource, Debug, Default, Clone)]
pub struct RewardRoomClaims {
    pub rooms: HashSet<RoomId>,
}

#[derive(Debug, Clone)]
pub enum RewardRoomAugmentService {
    Forge(Vec<AugmentChoiceOption>),
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
    ForgePick(Vec<AugmentChoiceOption>),
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
        .map(|registry| {
            build_sanctuary_draft(registry.augments.augments.as_slice(), &mut rng, inventory)
        })
        .unwrap_or_else(|| SanctuaryDraft {
            augment_service: RewardRoomAugmentService::Forge(Vec::new()),
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
    mut skill_choices: ResMut<SkillChoices>,
    data: Option<Res<GameDataRegistry>>,
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    floor: Option<Res<FloorNumber>>,
    mut player_q: Query<(&RewardModifiers, &mut Health, Option<&AugmentInventory>), With<Player>>,
) {
    let (Some(layout), Some(current)) = (layout.as_deref(), current.as_deref()) else {
        return;
    };

    if room_cleared
        .read()
        .find(|ev| ev.room == current.0)
        .is_none()
    {
        return;
    }

    pending_action.0 = None;

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

    if decision.heal_alive_fraction > 0.0
        && let Ok((_, mut health, _)) = player_q.get_single_mut()
    {
        let heal = health.max * decision.heal_alive_fraction;
        health.current = (health.current + heal).min(health.max);
    }

    let is_boss = room.room_type == RoomType::Boss;
    if !is_boss {
        let is_elite_room = room.room_type == RoomType::Elite;
        let should_offer_augment = is_elite_room || rng.gen_bool(0.40);
        if should_offer_augment && let Some(registry) = data.as_deref() {
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
        return;
    }

    flow.spawn_portal = true;
    flow.portal_is_victory = decision.post_reward == PostRewardDecision::Victory;
    augment_choices.options.clear();
    skill_choices.options.clear();

    if let Some(registry) = data.as_deref() {
        let inventory = player_q.get_single().ok().and_then(|(_, _, inv)| inv);
        let generated = generate_augment_choices(
            registry.augments.augments.as_slice(),
            &mut rng,
            true,
            inventory,
        );
        let generated_skills = generate_skill_choices(registry, &mut rng);
        if !generated.is_empty() {
            augment_choices.options = generated;
            augment_choices.return_state = Some(GamePhase::Playing);
        }
        if !generated_skills.is_empty() {
            skill_choices.options = generated_skills;
            skill_choices.return_state = if augment_choices.options.is_empty() {
                Some(GamePhase::Playing)
            } else {
                Some(GamePhase::AugmentSelect)
            };
            next_state.set(GamePhase::SkillSelect);
            return;
        }
        if !augment_choices.options.is_empty() {
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
        RewardFlowStep::ForgePick(options) => {
            if keyboard.just_pressed(KeyCode::Escape) {
                pending_action.0 = Some(RewardUiAction::Back);
                return;
            }
            if !options.is_empty()
                && (keyboard.just_pressed(KeyCode::Digit1)
                    || keyboard.just_pressed(KeyCode::Numpad1))
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
    mut commands: Commands,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut flow: ResMut<RewardFlow>,
    mut pending_action: ResMut<RewardPendingAction>,
    mut room_state: ResMut<RoomState>,
    mut cleared: EventWriter<RoomClearedEvent>,
    mut feedback: EventWriter<UiFeedbackEvent>,
    mut levelup_choices: ResMut<LevelUpChoices>,
    floor: Option<Res<FloorNumber>>,
    data: Option<Res<GameDataRegistry>>,
    mut rng: ResMut<GameRng>,
    mut player_q: ParamSet<(
        Query<(Entity, &mut Health, &mut Energy), With<Player>>,
        Query<&mut AugmentInventory, With<Player>>,
        Query<(&Health, &mut PlayerLevel, &mut Gold), With<Player>>,
    )>,
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
                if let Ok((entity, mut health, mut energy)) = player_q.p0().get_single_mut() {
                    let before_hp = health.current;
                    let before_energy = energy.current;
                    full_restore(&mut health, &mut energy);
                    commands.entity(entity).remove::<ArmorBroken>();
                    commands.entity(entity).remove::<Frozen>();
                    feedback.send(UiFeedbackEvent::card(
                        "圣所疗愈",
                        vec![
                            format!("HP: {:.0} -> {:.0}", before_hp, health.current),
                            format!("能量: {:.0} -> {:.0}", before_energy, energy.current),
                            "负面状态已清除。".to_string(),
                        ],
                        UiFeedbackSeverity::Success,
                        GamePhase::Playing,
                    ));
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
                let RewardRoomAugmentService::Forge(options) = sanctuary.augment_service;
                if options.is_empty() {
                    warn!("圣所锻造无可用选项，请检查 augments.ron");
                } else {
                    flow.step = RewardFlowStep::ForgePick(options);
                }
            }
            2 => {
                let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
                let default_scaling = RewardScalingConfig::default_config();
                let default_levelup = crate::data::definitions::LevelUpConfig::default_config();
                let (scaling, levelup) = data
                    .as_ref()
                    .map(|d| (&d.rewards.scaling, &d.rewards.levelup))
                    .unwrap_or((&default_scaling, &default_levelup));

                if let Ok((health, mut level, mut gold)) = player_q.p2().get_single_mut() {
                    let max_health = health.max;
                    let new_level = level.level + 1;
                    let curve = data
                        .as_deref()
                        .map(|d| d.economy.xp_curve.as_slice())
                        .unwrap_or(&[]);
                    apply_revelation_reward(&mut level, &mut gold, curve);
                    feedback.send(UiFeedbackEvent::toast(
                        "圣所启示",
                        vec![format!("等级提升到 {}，+50 金币。", new_level)],
                    ));
                    configure_revelation_choices(
                        &mut levelup_choices,
                        &mut rng,
                        scaling,
                        levelup,
                        max_health,
                        floor_number,
                        new_level,
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
        (RewardFlowStep::ForgePick(options), RewardUiAction::Back) => {
            // Return to the sanctuary preserving the SAME options. Never
            // rebuild via RNG here, otherwise Esc-then-reselect would let the
            // player re-roll the forge offers for free.
            flow.step = RewardFlowStep::Sanctuary(SanctuaryDraft {
                augment_service: RewardRoomAugmentService::Forge(options),
            });
        }
        (RewardFlowStep::ForgePick(options), RewardUiAction::Select(index)) => {
            let Some(choice) = options.get(index) else {
                return;
            };
            if let Ok(mut inventory) = player_q.p1().get_single_mut() {
                let grant = inventory.grant(choice.id);
                let mut lines = crate::ui::feedback::augment_grant_lines(grant, data.as_deref());
                lines.insert(0, "锻造完成。".to_string());
                feedback.send(UiFeedbackEvent::card(
                    "圣所锻造",
                    lines,
                    UiFeedbackSeverity::Success,
                    GamePhase::Playing,
                ));
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
    let forge_options = if upgrade_options.is_empty() {
        build_legendary_forge_choices(augments, rng, inventory)
    } else {
        upgrade_options
    };

    SanctuaryDraft {
        augment_service: RewardRoomAugmentService::Forge(forge_options),
    }
}

fn build_upgrade_candidates(
    augments: &[AugmentConfig],
    inventory: &AugmentInventory,
) -> Vec<AugmentChoiceOption> {
    inventory
        .augments
        .iter()
        .filter(|held| {
            augments
                .iter()
                .find(|augment| augment.id == held.id)
                .map(|augment| held.stacks < augment.max_stacks())
                .unwrap_or(false)
        })
        .filter_map(|held| {
            augments
                .iter()
                .find(|augment| augment.id == held.id)
                .map(|augment| AugmentChoiceOption {
                    id: augment.id,
                    title: augment.title.clone(),
                    description: augment.next_description(held.stacks).to_string(),
                    rarity: augment.rarity,
                    is_upgrade: true,
                })
        })
        .take(3)
        .collect()
}

fn build_legendary_forge_choices(
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
        .filter(|augment| matches!(augment.rarity, AugmentRarity::Legendary))
        .filter(|augment| !owned.contains(&augment.id))
        .collect::<Vec<_>>();
    if pool.is_empty() {
        pool = augments
            .iter()
            .filter(|augment| matches!(augment.rarity, AugmentRarity::Legendary))
            .collect::<Vec<_>>();
    }
    if pool.is_empty() {
        pool = augments
            .iter()
            .filter(|augment| {
                matches!(
                    augment.rarity,
                    AugmentRarity::Elite | AugmentRarity::Legendary
                )
            })
            .filter(|augment| !owned.contains(&augment.id))
            .collect::<Vec<_>>();
    }
    if pool.is_empty() {
        pool = augments
            .iter()
            .filter(|augment| {
                matches!(
                    augment.rarity,
                    AugmentRarity::Elite | AugmentRarity::Legendary
                )
            })
            .collect::<Vec<_>>();
    }
    rng.shuffle(&mut pool);
    pool.into_iter()
        .take(2)
        .map(|augment| AugmentChoiceOption {
            id: augment.id,
            title: augment.title.clone(),
            description: augment.description_for_stacks(1).to_string(),
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
    let is_maxed = |augment: &AugmentConfig| {
        inventory
            .map(|inv| inv.stacks(augment.id) >= augment.max_stacks())
            .unwrap_or(false)
    };
    let pool: Vec<_> = augments
        .iter()
        .filter(|augment| !is_maxed(augment))
        .filter(|augment| {
            if is_boss {
                matches!(
                    augment.rarity,
                    AugmentRarity::Elite | AugmentRarity::Legendary
                )
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
            let held_stacks = inventory.map(|inv| inv.stacks(augment.id)).unwrap_or(0);
            let is_upgrade = held_stacks > 0 && held_stacks < augment.max_stacks();
            let description = if is_upgrade {
                augment.next_description(held_stacks)
            } else {
                augment.description_for_stacks(1)
            };
            AugmentChoiceOption {
                id: augment.id,
                title: augment.title.clone(),
                description: description.to_string(),
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
    levelup: &crate::data::definitions::LevelUpConfig,
    max_health: f32,
    floor_number: u32,
    new_level: u32,
) {
    choices.options = build_levelup_options(rng, scaling, levelup, max_health, floor_number);
    choices.return_state = Some(GamePhase::Playing);
    choices.new_level = new_level;
    choices.crit_cap = levelup.crit_cap;
    choices.melee_min_s = levelup.melee_min_s;
    choices.ranged_min_s = levelup.ranged_min_s;
    choices.dash_min_s = levelup.dash_min_s;
}

fn generate_skill_choices(
    registry: &GameDataRegistry,
    rng: &mut GameRng,
) -> Vec<SkillChoiceOption> {
    let mut skills = registry.skills.skills.iter().collect::<Vec<_>>();
    rng.shuffle(&mut skills);
    skills
        .into_iter()
        .take(3)
        .map(|skill| SkillChoiceOption {
            skill: skill.skill,
            title: skill.title.clone(),
            description: skill.description.clone(),
            energy_cost: skill.energy_cost,
            cooldown_s: skill.cooldown_s,
        })
        .collect()
}

fn full_restore(health: &mut Health, energy: &mut Energy) {
    health.current = health.max;
    energy.current = energy.max;
}

fn apply_revelation_reward(level: &mut PlayerLevel, gold: &mut Gold, curve: &[u32]) {
    level.level += 1;
    level.xp_to_next = crate::gameplay::progression::experience::xp_threshold(curve, level.level);
    gold.0 = gold.0.saturating_add(50);
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
    use crate::data::definitions::{AugmentConfig, AugmentLevelConfig};
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
            levels: vec![
                AugmentLevelConfig {
                    description: description.to_string(),
                    params: Default::default(),
                },
                AugmentLevelConfig {
                    description: upgraded_description.to_string(),
                    params: Default::default(),
                },
                AugmentLevelConfig {
                    description: format!("{upgraded_description}（质变）"),
                    params: Default::default(),
                },
            ],
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
            sample_augment(
                AugmentId::CritEnhance,
                AugmentRarity::Legendary,
                "弱点洞察",
                "暴击更高",
                "暴击更强",
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
    fn upgrade_service_lists_non_maxed_augments() {
        let inventory = AugmentInventory {
            augments: vec![
                HeldAugment {
                    id: AugmentId::GoldBonus,
                    stacks: 1,
                },
                HeldAugment {
                    id: AugmentId::Thorns,
                    stacks: 2,
                },
                HeldAugment {
                    id: AugmentId::Phoenix,
                    stacks: 3,
                },
            ],
        };

        let options = build_upgrade_candidates(&sample_augments(), &inventory);

        assert_eq!(options.len(), 2);
        assert_eq!(options[0].id, AugmentId::GoldBonus);
        assert_eq!(options[1].id, AugmentId::Thorns);
        assert!(options.iter().all(|option| option.is_upgrade));
    }

    #[test]
    fn sanctuary_forge_offers_legendary_candidates_when_no_upgrade_exists() {
        let mut rng = seeded_rng(9);
        let inventory = AugmentInventory::default();

        let draft = build_sanctuary_draft(&sample_augments(), &mut rng, Some(&inventory));

        match draft.augment_service {
            RewardRoomAugmentService::Forge(options) => {
                assert_eq!(options.len(), 2);
                assert!(
                    options
                        .iter()
                        .all(|option| option.rarity == AugmentRarity::Legendary)
                );
            }
        }
    }

    #[test]
    fn revelation_grants_level_and_gold() {
        const CURVE: &[u32] = &[50, 70, 90, 110, 130, 150, 180, 200, 220];
        let mut level = PlayerLevel {
            level: 2,
            xp: 0,
            xp_to_next: crate::gameplay::progression::experience::xp_threshold(CURVE, 2),
        };
        let mut gold = Gold(20);

        apply_revelation_reward(&mut level, &mut gold, CURVE);

        assert_eq!(level.level, 3);
        assert_eq!(
            level.xp_to_next,
            crate::gameplay::progression::experience::xp_threshold(CURVE, 3)
        );
        assert_eq!(gold.0, 70);
    }

    #[test]
    fn revelation_choices_return_to_ingame() {
        let mut rng = seeded_rng(11);
        let mut choices = LevelUpChoices::default();

        configure_revelation_choices(
            &mut choices,
            &mut rng,
            &RewardScalingConfig::default_config(),
            &crate::data::definitions::LevelUpConfig::default_config(),
            100.0,
            1,
            3,
        );

        assert_eq!(choices.return_state, Some(GamePhase::Playing));
        assert_eq!(choices.new_level, 3);
        assert_eq!(choices.options.len(), 4);
    }
}
