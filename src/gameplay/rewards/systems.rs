use bevy::prelude::*;
use bevy::utils::HashSet;

use crate::core::assets::GameAssets;
use crate::core::events::{
    RewardChoiceGroup, RewardChosenEvent, RoomClearedEvent, SpawnEnemyEvent,
};
use crate::data::definitions::RewardScalingConfig;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentInventory, AugmentRarity};
use crate::gameplay::curse::CurseState;
use crate::gameplay::enemy::systems::{ClearGrace, EnemySpawnCount, SpawnedForRoom};
use crate::gameplay::map::generator::{build_rooms, reset_player_for_floor, spawn_current_room};
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId, RoomType};
use crate::gameplay::map::transitions::RoomTransition;
use crate::gameplay::map::{InGameEntity, VisitedRooms};
use crate::gameplay::player::components::{
    AttackCooldown, AttackPower, CritChance, DashCooldown, DashState, Energy, Health, MoveSpeed,
    Player, RangedCooldown, RewardModifiers, Velocity,
};
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::rewards::data::RewardType;
use crate::gameplay::rune::data::RuneLoadout;
use crate::gameplay::session_core::{
    BlessingOffer, PlayerRuleEffects, PlayerRuleSnapshot, PostRewardDecision, RewardDraft,
    RewardDraftMode, RewardOptionDraft, RewardSelection, SessionMode, SessionRuleContext,
    apply_reward_selection as apply_shared_reward_selection, build_reward_draft,
    generate_blessing_choices, on_room_cleared, on_room_enter,
};
use crate::states::{AppState, RoomState};
use crate::ui::augment_select::{AugmentChoiceOption, AugmentChoices};
use crate::utils::entity::safe_despawn_recursive;
use crate::utils::rng::GameRng;

#[derive(Resource, Debug, Default, Clone)]
pub struct RewardChoices {
    pub primary: Vec<RewardType>,
    pub secondary: Vec<RewardType>,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct RewardRoomClaims {
    pub rooms: HashSet<RoomId>,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct BlessingFlow {
    pub offers: Vec<BlessingOffer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlessingUiAction {
    Select(usize),
    Leave,
}

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BlessingPendingAction(pub Option<BlessingUiAction>);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RewardFlowMode {
    #[default]
    SingleBuff,
    HealOrBuff,
    DualBuff,
    Blessing,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct RewardFlow {
    pub mode: RewardFlowMode,
    pub spawn_portal: bool,
    pub portal_is_victory: bool,
    pub reward_scale: f32,
    pub selected_primary: Option<RewardType>,
    pub selected_secondary: Option<RewardType>,
}

#[derive(Component)]
pub struct BossPortal {
    pub is_victory: bool,
}

pub struct RewardsSystemsPlugin;

impl Plugin for RewardsSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RewardChoices>()
            .init_resource::<RewardRoomClaims>()
            .init_resource::<BlessingFlow>()
            .init_resource::<BlessingPendingAction>()
            .init_resource::<RewardFlow>()
            .init_resource::<GameRng>()
            .add_systems(
                Update,
                enter_reward_selection.run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                offer_reward_in_reward_room.run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (spawn_boss_portal, boss_portal_interact).run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                OnEnter(AppState::RewardSelect),
                crate::ui::reward_select::setup_reward_ui,
            )
            .add_systems(
                Update,
                (
                    handle_reward_choice_input,
                    crate::ui::reward_select::reward_ui_input_system,
                    crate::ui::reward_select::update_reward_ui,
                )
                    .run_if(in_state(AppState::RewardSelect)),
            )
            .add_systems(
                OnExit(AppState::RewardSelect),
                crate::ui::reward_select::cleanup_reward_ui,
            )
            .add_systems(
                Update,
                apply_reward_choice
                    .run_if(in_state(AppState::RewardSelect))
                    .after(handle_reward_choice_input)
                    .after(crate::ui::reward_select::reward_ui_input_system),
            );
    }
}

fn offer_reward_in_reward_room(
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    data: Option<Res<GameDataRegistry>>,
    floor: Option<Res<FloorNumber>>,
    transition: Option<Res<RoomTransition>>,
    mut claims: ResMut<RewardRoomClaims>,
    mut choices: ResMut<RewardChoices>,
    mut blessing_flow: ResMut<BlessingFlow>,
    mut blessing_pending: ResMut<BlessingPendingAction>,
    mut rng: ResMut<GameRng>,
    mut flow: ResMut<RewardFlow>,
    mut next_state: ResMut<NextState<AppState>>,
    player_q: Query<(&RuneLoadout, &CurseState), With<Player>>,
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

    reset_reward_flow(&mut flow);
    reset_blessing_flow(&mut blessing_flow, &mut blessing_pending);
    choices.primary.clear();
    choices.secondary.clear();
    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
    let (rune_loadout, has_active_curse) = player_q
        .get_single()
        .map(|(loadout, curses)| (loadout.clone(), curses.has_any_curse()))
        .unwrap_or((RuneLoadout::default(), false));
    let mode = on_room_enter(
        SessionRuleContext {
            mode: SessionMode::Solo,
            floor: floor_number,
            total_floors: data
                .as_deref()
                .map(|registry| registry.balance.total_floors.max(1))
                .unwrap_or(4),
            boss_gives_victory: data
                .as_deref()
                .map(|registry| registry.balance.boss_room_gives_victory)
                .unwrap_or(false),
            room_type: room.room_type,
        },
        true,
        has_active_curse,
    )
    .reward_mode;
    let Some(mode) = mode else {
        return;
    };

    flow.mode = reward_flow_mode_from_draft(mode);
    flow.reward_scale = reward_scale_for_draft(mode);

    if mode == RewardDraftMode::Blessing {
        let Some(data) = data.as_deref() else {
            return;
        };
        blessing_flow.offers = generate_blessing_choices(
            &mut rng,
            floor_number,
            &rune_loadout,
            &data.runes,
            &data.curses,
        );
        if blessing_flow.offers.is_empty() {
            return;
        }
    } else {
        let draft = build_reward_draft(
            SessionMode::Solo,
            mode,
            &mut rng,
            &[PlayerRuleSnapshot {
                player_index: 0,
                alive: true,
                mods: RewardModifiers::default(),
            }],
        );
        apply_solo_reward_draft(&draft, &mut choices);
    }
    next_state.set(AppState::RewardSelect);
}

fn enter_reward_selection(
    mut room_cleared: EventReader<RoomClearedEvent>,
    mut next_state: ResMut<NextState<AppState>>,
    mut choices: ResMut<RewardChoices>,
    mut blessing_flow: ResMut<BlessingFlow>,
    mut blessing_pending: ResMut<BlessingPendingAction>,
    mut rng: ResMut<GameRng>,
    mut flow: ResMut<RewardFlow>,
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

    reset_reward_flow(&mut flow);
    reset_blessing_flow(&mut blessing_flow, &mut blessing_pending);
    choices.primary.clear();
    choices.secondary.clear();

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

    // Apply healing if any (Boss rooms heal 80%)
    if decision.heal_alive_fraction > 0.0 {
        if let Ok((_, mut health, _)) = player_q.get_single_mut() {
            let heal = health.max * decision.heal_alive_fraction;
            health.current = (health.current + heal).min(health.max);
        }
    }

    let is_boss = room.room_type == RoomType::Boss;

    // Non-boss rooms: no RewardSelect; elite rooms always offer AugmentSelect.
    if !is_boss {
        let is_elite_room = room.room_type == RoomType::Elite;
        let should_offer_augment = is_elite_room || rng.gen_bool(0.40);
        if should_offer_augment {
            if let Some(registry) = data.as_deref() {
                let inventory = player_q.get_single().ok().and_then(|(_, _, inv)| inv);
                let generated = generate_augment_choices(registry, &mut rng, false, inventory);
                if !generated.is_empty() {
                    augment_choices.options = generated;
                    augment_choices.return_state = Some(AppState::InGame);
                    next_state.set(AppState::AugmentSelect);
                }
            }
        }
        // No RewardSelect for non-boss rooms — XP/gold already given on kill
        return;
    }

    flow.spawn_portal = true;
    flow.portal_is_victory = decision.post_reward == PostRewardDecision::Victory;

    if let Some(registry) = data.as_deref() {
        let inventory = player_q.get_single().ok().and_then(|(_, _, inv)| inv);
        let generated = generate_augment_choices(registry, &mut rng, true, inventory);
        if !generated.is_empty() {
            augment_choices.options = generated;
            augment_choices.return_state = Some(AppState::InGame);
            next_state.set(AppState::AugmentSelect);
            return;
        }
    }

    next_state.set(AppState::InGame);
}

fn handle_reward_choice_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<RewardChosenEvent>,
    choices: Res<RewardChoices>,
    flow: Res<RewardFlow>,
    mut blessing_pending: ResMut<BlessingPendingAction>,
) {
    if flow.mode == RewardFlowMode::Blessing {
        if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1) {
            blessing_pending.0 = Some(BlessingUiAction::Select(0));
        } else if keyboard.just_pressed(KeyCode::Digit2) || keyboard.just_pressed(KeyCode::Numpad2)
        {
            blessing_pending.0 = Some(BlessingUiAction::Select(1));
        } else if keyboard.just_pressed(KeyCode::Escape) {
            blessing_pending.0 = Some(BlessingUiAction::Leave);
        }
        return;
    }

    let mapped = match flow.mode {
        RewardFlowMode::SingleBuff => map_reward_key(
            &keyboard,
            RewardChoiceGroup::Primary,
            &choices.primary,
            [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3],
            [KeyCode::Numpad1, KeyCode::Numpad2, KeyCode::Numpad3],
        ),
        RewardFlowMode::HealOrBuff => {
            if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1) {
                Some((RewardChoiceGroup::Heal, RewardType::RecoverHealth))
            } else {
                map_reward_key(
                    &keyboard,
                    RewardChoiceGroup::Primary,
                    &choices.primary,
                    [KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4],
                    [KeyCode::Numpad2, KeyCode::Numpad3, KeyCode::Numpad4],
                )
            }
        }
        RewardFlowMode::DualBuff => map_reward_key(
            &keyboard,
            RewardChoiceGroup::Primary,
            &choices.primary,
            [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3],
            [KeyCode::Numpad1, KeyCode::Numpad2, KeyCode::Numpad3],
        )
        .or_else(|| {
            map_reward_key(
                &keyboard,
                RewardChoiceGroup::Secondary,
                &choices.secondary,
                [KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6],
                [KeyCode::Numpad4, KeyCode::Numpad5, KeyCode::Numpad6],
            )
        }),
        RewardFlowMode::Blessing => None,
    };

    if let Some((group, reward)) = mapped {
        events.send(RewardChosenEvent { reward, group });
    }
}

fn apply_reward_choice(
    mut chosen: EventReader<RewardChosenEvent>,
    mut next_state: ResMut<NextState<AppState>>,
    mut flow: ResMut<RewardFlow>,
    mut choices: ResMut<RewardChoices>,
    mut blessing_flow: ResMut<BlessingFlow>,
    mut blessing_pending: ResMut<BlessingPendingAction>,
    mut player_q: Query<
        (
            &mut RewardModifiers,
            &mut Health,
            &mut Energy,
            &mut MoveSpeed,
            &mut DashCooldown,
            &mut RangedCooldown,
            &mut CritChance,
            &mut AttackCooldown,
            &mut AttackPower,
            &mut RuneLoadout,
            &mut CurseState,
        ),
        With<Player>,
    >,
    floor: Option<Res<FloorNumber>>,
    registry: Option<Res<GameDataRegistry>>,
) {
    if flow.mode == RewardFlowMode::Blessing {
        let Some(action) = blessing_pending.0.take() else {
            return;
        };

        if let BlessingUiAction::Select(index) = action {
            if let Some(offer) = blessing_flow.offers.get(index).cloned() {
                if let Ok((
                    _mods,
                    _health,
                    _energy,
                    _move_speed,
                    _dash_cd,
                    _ranged_cd,
                    _crit,
                    _atk_cd,
                    _attack_power,
                    mut rune_loadout,
                    mut curse_state,
                )) = player_q.get_single_mut()
                {
                    rune_loadout.equip(offer.rune_slot, offer.rune_id);
                    curse_state.add_curse(offer.curse_id, offer.curse_duration);
                } else {
                    warn!("blessing chosen but no player entity was found");
                }
            }
        }

        reset_reward_flow(&mut flow);
        reset_blessing_flow(&mut blessing_flow, &mut blessing_pending);
        choices.primary.clear();
        choices.secondary.clear();
        next_state.set(AppState::InGame);
        return;
    }

    for ev in chosen.read() {
        let choice_valid = match flow.mode {
            RewardFlowMode::SingleBuff => ev.group == RewardChoiceGroup::Primary,
            RewardFlowMode::HealOrBuff => {
                ev.group == RewardChoiceGroup::Heal || ev.group == RewardChoiceGroup::Primary
            }
            RewardFlowMode::DualBuff => match ev.group {
                RewardChoiceGroup::Primary => flow.selected_primary.is_none(),
                RewardChoiceGroup::Secondary => flow.selected_secondary.is_none(),
                RewardChoiceGroup::Heal => false,
            },
            RewardFlowMode::Blessing => false,
        };
        if !choice_valid {
            continue;
        }

        let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
        if let Ok((
            mut mods,
            mut health,
            mut energy,
            mut move_speed,
            mut dash_cd,
            mut ranged_cd,
            mut crit,
            mut atk_cd,
            mut attack_power,
            _rune_loadout,
            _curse_state,
        )) = player_q.get_single_mut()
        {
            let selection = RewardSelection {
                mode: reward_draft_mode_from_flow(flow.mode),
                primary: match ev.group {
                    RewardChoiceGroup::Heal | RewardChoiceGroup::Primary => {
                        Some(reward_option_from_choice(ev.reward, ev.group))
                    }
                    RewardChoiceGroup::Secondary => None,
                },
                secondary: match ev.group {
                    RewardChoiceGroup::Secondary => {
                        Some(reward_option_from_choice(ev.reward, ev.group))
                    }
                    _ => None,
                },
            };
            let mut effects = PlayerRuleEffects {
                health: &mut health,
                energy: &mut energy,
                move_speed: &mut move_speed,
                attack_power: &mut attack_power,
                crit: &mut crit,
                dash_cooldown: &mut dash_cd,
                attack_cooldown: &mut atk_cd,
                ranged_cooldown: &mut ranged_cd,
                mods: &mut mods,
            };
            let scaling = registry
                .as_ref()
                .map(|d| d.rewards.scaling.clone())
                .unwrap_or_else(RewardScalingConfig::default_config);
            let _ = apply_shared_reward_selection(
                selection,
                floor_number,
                &mut effects,
                PostRewardDecision::ResumeRun,
                &scaling,
            );
        } else {
            warn!("reward chosen but no player entity was found");
        }

        match flow.mode {
            RewardFlowMode::SingleBuff | RewardFlowMode::HealOrBuff => {}
            RewardFlowMode::DualBuff => match ev.group {
                RewardChoiceGroup::Primary => flow.selected_primary = Some(ev.reward),
                RewardChoiceGroup::Secondary => flow.selected_secondary = Some(ev.reward),
                RewardChoiceGroup::Heal => {}
            },
            RewardFlowMode::Blessing => {}
        }

        if flow.mode == RewardFlowMode::DualBuff
            && (flow.selected_primary.is_none() || flow.selected_secondary.is_none())
        {
            continue;
        }

        flow.mode = RewardFlowMode::SingleBuff;
        flow.reward_scale = 1.0;
        flow.selected_primary = None;
        flow.selected_secondary = None;
        reset_blessing_flow(&mut blessing_flow, &mut blessing_pending);
        choices.primary.clear();
        choices.secondary.clear();

        next_state.set(AppState::InGame);
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
    mut next_state: ResMut<NextState<AppState>>,
    ingame_entities: Query<(Entity, Option<&Player>), With<InGameEntity>>,
    mut floor: Option<ResMut<FloorNumber>>,
    mut spawned_for_room: ResMut<SpawnedForRoom>,
    mut grace: ResMut<ClearGrace>,
    mut spawn_count: ResMut<EnemySpawnCount>,
    mut rng: ResMut<GameRng>,
    data: Option<Res<GameDataRegistry>>,
    visited: Option<ResMut<VisitedRooms>>,
    player_curse_q: Query<&CurseState, With<Player>>,
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
        next_state.set(AppState::Victory);
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
    let has_active_curse = player_curse_q
        .get_single()
        .map(CurseState::has_any_curse)
        .unwrap_or(false);
    let layout = FloorLayout {
        rooms: build_rooms(data.as_deref(), floor_number, has_active_curse, &mut rng),
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

    next_state.set(AppState::InGame);
}

fn map_reward_key(
    keyboard: &ButtonInput<KeyCode>,
    group: RewardChoiceGroup,
    choices: &[RewardType],
    digits: [KeyCode; 3],
    numpads: [KeyCode; 3],
) -> Option<(RewardChoiceGroup, RewardType)> {
    for (idx, reward) in choices.iter().copied().enumerate() {
        if keyboard.just_pressed(digits[idx]) || keyboard.just_pressed(numpads[idx]) {
            return Some((group, reward));
        }
    }
    None
}

fn reset_reward_flow(flow: &mut RewardFlow) {
    flow.spawn_portal = false;
    flow.portal_is_victory = false;
    flow.mode = RewardFlowMode::SingleBuff;
    flow.reward_scale = 1.0;
    flow.selected_primary = None;
    flow.selected_secondary = None;
}

fn reset_blessing_flow(
    blessing_flow: &mut BlessingFlow,
    blessing_pending: &mut BlessingPendingAction,
) {
    blessing_flow.offers.clear();
    blessing_pending.0 = None;
}

fn reward_flow_mode_from_draft(mode: RewardDraftMode) -> RewardFlowMode {
    match mode {
        RewardDraftMode::SingleBuff | RewardDraftMode::LoneSurvivor => RewardFlowMode::SingleBuff,
        RewardDraftMode::HealOrBuff => RewardFlowMode::HealOrBuff,
        RewardDraftMode::DualBuff => RewardFlowMode::DualBuff,
        RewardDraftMode::Blessing => RewardFlowMode::Blessing,
    }
}

fn reward_draft_mode_from_flow(mode: RewardFlowMode) -> RewardDraftMode {
    match mode {
        RewardFlowMode::SingleBuff => RewardDraftMode::SingleBuff,
        RewardFlowMode::HealOrBuff => RewardDraftMode::HealOrBuff,
        RewardFlowMode::DualBuff => RewardDraftMode::DualBuff,
        RewardFlowMode::Blessing => RewardDraftMode::Blessing,
    }
}

fn reward_scale_for_draft(mode: RewardDraftMode) -> f32 {
    match mode {
        RewardDraftMode::DualBuff => 1.50,
        _ => 1.0,
    }
}

fn apply_solo_reward_draft(draft: &RewardDraft, choices: &mut RewardChoices) {
    let Some(player) = draft.players.first() else {
        choices.primary.clear();
        choices.secondary.clear();
        return;
    };
    choices.primary = player
        .primary_options
        .iter()
        .filter_map(|option| match option {
            RewardOptionDraft::Buff(reward) => Some(*reward),
            _ => None,
        })
        .collect();
    choices.secondary = player
        .secondary_options
        .iter()
        .filter_map(|option| match option {
            RewardOptionDraft::Buff(reward) => Some(*reward),
            _ => None,
        })
        .collect();
}

fn reward_option_from_choice(reward: RewardType, group: RewardChoiceGroup) -> RewardOptionDraft {
    if group == RewardChoiceGroup::Heal || reward == RewardType::RecoverHealth {
        RewardOptionDraft::Rest
    } else {
        RewardOptionDraft::Buff(reward)
    }
}

/// Generate 3 augment choices from the registry, filtered by rarity pool.
/// Boss rooms offer Elite+Legendary; normal rooms offer Common(+small Elite chance).
fn generate_augment_choices(
    registry: &GameDataRegistry,
    rng: &mut GameRng,
    is_boss: bool,
    inventory: Option<&AugmentInventory>,
) -> Vec<AugmentChoiceOption> {
    let pool: Vec<_> = registry
        .augments
        .augments
        .iter()
        .filter(|a| {
            if is_boss {
                matches!(a.rarity, AugmentRarity::Elite | AugmentRarity::Legendary)
            } else {
                // Normal rooms: mostly common, 20% chance to include elite
                matches!(a.rarity, AugmentRarity::Common | AugmentRarity::Elite)
            }
        })
        .collect();

    if pool.is_empty() {
        return vec![];
    }

    let mut indices: Vec<usize> = (0..pool.len()).collect();
    rng.shuffle(&mut indices);
    indices.truncate(3);

    indices
        .iter()
        .map(|&i| {
            let a = &pool[i];
            let is_upgrade = inventory.map(|inv| inv.has(a.id)).unwrap_or(false);
            let desc = if is_upgrade {
                &a.upgraded_description
            } else {
                &a.description
            };
            AugmentChoiceOption {
                id: a.id,
                title: a.title.clone(),
                description: desc.clone(),
                rarity: a.rarity,
                is_upgrade,
            }
        })
        .collect()
}
