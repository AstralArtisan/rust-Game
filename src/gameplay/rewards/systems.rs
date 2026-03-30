use bevy::prelude::*;
use bevy::utils::HashSet;

use crate::core::events::{RewardChoiceGroup, RewardChosenEvent, RoomClearedEvent};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::enemy::systems::{ClearGrace, EnemySpawnCount, SpawnedForRoom};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId, RoomType};
use crate::gameplay::map::transitions::RoomTransition;
use crate::gameplay::player::components::{
    AttackCooldown, AttackPower, CritChance, DashCooldown, Energy, Health, MoveSpeed, Player,
    RangedCooldown, RewardModifiers,
};
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::rewards::data::RewardType;
use crate::gameplay::session_core::{
    PlayerRuleEffects, PlayerRuleSnapshot, PostRewardDecision, RewardDraft, RewardDraftMode,
    RewardOptionDraft, RewardSelection, SessionMode, SessionRuleContext,
    apply_reward_selection as apply_shared_reward_selection, build_reward_draft, on_room_cleared,
    on_room_enter,
};
use crate::states::{AppState, RoomState};
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RewardFlowMode {
    #[default]
    SingleBuff,
    HealOrBuff,
    DualBuff,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct RewardFlow {
    pub mode: RewardFlowMode,
    pub go_next_floor: bool,
    pub go_victory: bool,
    pub reward_scale: f32,
    pub selected_primary: Option<RewardType>,
    pub selected_secondary: Option<RewardType>,
}

pub struct RewardsSystemsPlugin;

impl Plugin for RewardsSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RewardChoices>()
            .init_resource::<RewardRoomClaims>()
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
    transition: Option<Res<RoomTransition>>,
    mut claims: ResMut<RewardRoomClaims>,
    mut choices: ResMut<RewardChoices>,
    mut rng: ResMut<GameRng>,
    mut flow: ResMut<RewardFlow>,
    mut next_state: ResMut<NextState<AppState>>,
    player_q: Query<&RewardModifiers, With<Player>>,
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
    let mode = on_room_enter(
        SessionRuleContext {
            mode: SessionMode::Solo,
            floor: 1,
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
    )
    .reward_mode;
    let Some(mode) = mode else {
        return;
    };

    flow.mode = reward_flow_mode_from_draft(mode);
    flow.reward_scale = reward_scale_for_draft(mode);

    let mods = player_q.get_single().copied().unwrap_or_default();
    let draft = build_reward_draft(
        SessionMode::Solo,
        mode,
        &mut rng,
        &[PlayerRuleSnapshot {
            player_index: 0,
            alive: true,
            mods,
        }],
    );
    apply_solo_reward_draft(&draft, &mut choices);
    next_state.set(AppState::RewardSelect);
}

fn enter_reward_selection(
    mut room_cleared: EventReader<RoomClearedEvent>,
    mut next_state: ResMut<NextState<AppState>>,
    mut choices: ResMut<RewardChoices>,
    mut rng: ResMut<GameRng>,
    mut flow: ResMut<RewardFlow>,
    data: Option<Res<GameDataRegistry>>,
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    floor: Option<Res<FloorNumber>>,
    mut player_q: Query<(&RewardModifiers, &mut Health), With<Player>>,
) {
    let Some(ev) = room_cleared.read().next() else {
        return;
    };

    reset_reward_flow(&mut flow);

    let (Some(layout), Some(current)) = (layout.as_deref(), current.as_deref()) else {
        return;
    };
    if ev.room != current.0 {
        return;
    }

    let Some(room) = layout.room(current.0) else {
        return;
    };
    if room.room_type == RoomType::Reward {
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
    let Some(mode) = decision.reward_mode else {
        return;
    };

    if decision.heal_alive_fraction > 0.0 {
        if let Ok((_, mut health)) = player_q.get_single_mut() {
            let heal = health.max * decision.heal_alive_fraction;
            health.current = (health.current + heal).min(health.max);
        }
    }

    flow.mode = reward_flow_mode_from_draft(mode);
    flow.reward_scale = reward_scale_for_draft(mode);
    set_post_reward_flags(&mut flow, decision.post_reward);

    let mods = player_q
        .get_single()
        .map(|(mods, _)| *mods)
        .unwrap_or_default();
    let draft = build_reward_draft(
        SessionMode::Solo,
        mode,
        &mut rng,
        &[PlayerRuleSnapshot {
            player_index: 0,
            alive: true,
            mods,
        }],
    );
    apply_solo_reward_draft(&draft, &mut choices);
    next_state.set(AppState::RewardSelect);
}

fn handle_reward_choice_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<RewardChosenEvent>,
    choices: Res<RewardChoices>,
    flow: Res<RewardFlow>,
) {
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
        ),
        With<Player>,
    >,
    mut commands: Commands,
    ingame_entities: Query<(Entity, Option<&Player>), With<InGameEntity>>,
    mut floor: Option<ResMut<FloorNumber>>,
    mut spawned_for_room: ResMut<SpawnedForRoom>,
    mut grace: ResMut<ClearGrace>,
    mut spawn_count: ResMut<EnemySpawnCount>,
) {
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
            let _ = apply_shared_reward_selection(
                selection,
                floor_number,
                &mut effects,
                PostRewardDecision::ResumeRun,
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
        choices.primary.clear();
        choices.secondary.clear();

        if flow.go_next_floor {
            for (entity, player) in &ingame_entities {
                if player.is_none() {
                    safe_despawn_recursive(&mut commands, entity);
                }
            }

            commands.remove_resource::<FloorLayout>();
            commands.remove_resource::<CurrentRoom>();
            commands.remove_resource::<RoomTransition>();
            commands.remove_resource::<RoomState>();

            if let Some(floor) = floor.as_mut() {
                floor.0 += 1;
            }

            spawned_for_room.0 = None;
            grace.last_room = None;
            grace.timer = Timer::from_seconds(0.0, TimerMode::Once);
            spawn_count.current = 0;
            flow.go_next_floor = false;
        }

        if flow.go_victory {
            flow.go_victory = false;
            flow.go_next_floor = false;
            next_state.set(AppState::Victory);
        } else {
            next_state.set(AppState::InGame);
        }
    }
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
    flow.go_next_floor = false;
    flow.go_victory = false;
    flow.mode = RewardFlowMode::SingleBuff;
    flow.reward_scale = 1.0;
    flow.selected_primary = None;
    flow.selected_secondary = None;
}

fn reward_flow_mode_from_draft(mode: RewardDraftMode) -> RewardFlowMode {
    match mode {
        RewardDraftMode::SingleBuff | RewardDraftMode::LoneSurvivor => RewardFlowMode::SingleBuff,
        RewardDraftMode::HealOrBuff => RewardFlowMode::HealOrBuff,
        RewardDraftMode::DualBuff => RewardFlowMode::DualBuff,
    }
}

fn reward_draft_mode_from_flow(mode: RewardFlowMode) -> RewardDraftMode {
    match mode {
        RewardFlowMode::SingleBuff => RewardDraftMode::SingleBuff,
        RewardFlowMode::HealOrBuff => RewardDraftMode::HealOrBuff,
        RewardFlowMode::DualBuff => RewardDraftMode::DualBuff,
    }
}

fn reward_scale_for_draft(mode: RewardDraftMode) -> f32 {
    match mode {
        RewardDraftMode::DualBuff => 1.50,
        _ => 1.0,
    }
}

fn set_post_reward_flags(flow: &mut RewardFlow, post_reward: PostRewardDecision) {
    match post_reward {
        PostRewardDecision::ResumeRun => {
            flow.go_next_floor = false;
            flow.go_victory = false;
        }
        PostRewardDecision::NextFloor => {
            flow.go_next_floor = true;
            flow.go_victory = false;
        }
        PostRewardDecision::Victory => {
            flow.go_next_floor = false;
            flow.go_victory = true;
        }
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
