use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::constants::{ROOM_HALF_HEIGHT, ROOM_HALF_WIDTH};
use crate::coop::components::{CoopPhase, CoopSessionEntity, CoopSessionState};
use crate::coop::components::{GhostState, LocalControlled};
use crate::core::assets::GameAssets;
use crate::core::events::DeathEvent;
use crate::core::input::PlayerInputState;
use crate::core::test_mode::TestMode;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::AugmentInventory;
use crate::gameplay::augment::effects::DashResetSpeedBuff;
use crate::gameplay::combat::components::{Hurtbox, Knockback, Team};
use crate::gameplay::effects::flash::Flash;
use crate::gameplay::map::InGameEntity;
use crate::gameplay::progression::experience::PlayerLevel;
use crate::gameplay::session_core::{DeathDecision, SessionMode, evaluate_death};
use crate::states::{GamePhase, RoomState};
use crate::utils::math::{clamp_in_room, clamp_length};

use super::animation::PlayerAnim;
use super::components::*;

pub fn spawn_player(
    mut commands: Commands,
    assets: Res<GameAssets>,
    data: Option<Res<GameDataRegistry>>,
    existing_player_q: Query<(), With<Player>>,
) {
    if existing_player_q.iter().next().is_some() {
        return;
    }

    let cfg = data.as_deref().map(|d| &d.player);
    let max_hp = cfg.map(|c| c.max_hp).unwrap_or(100.0);
    let move_speed = cfg.map(|c| c.move_speed).unwrap_or(260.0);
    let attack_power = cfg.map(|c| c.attack_power).unwrap_or(18.0);
    let attack_cd = cfg.map(|c| c.attack_cooldown_s).unwrap_or(0.70);
    let ranged_cd = cfg.map(|c| c.ranged_cooldown_s).unwrap_or(0.80);
    let dash_cd = cfg.map(|c| c.dash_cooldown_s).unwrap_or(1.2);
    let dash_speed = cfg.map(|c| c.dash_speed).unwrap_or(680.0);
    let dash_duration = cfg.map(|c| c.dash_duration_s).unwrap_or(0.12);
    let inv_s = cfg.map(|c| c.invincibility_s).unwrap_or(0.35);
    let crit = cfg.map(|c| c.crit_chance).unwrap_or(0.05);
    let energy_max = cfg.map(|c| c.energy_max).unwrap_or(100.0);
    let skill1_cd = cfg.map(|c| c.skill1_cooldown_s).unwrap_or(1.1);

    let mut entity = commands.spawn((SpriteBundle {
        texture: assets.textures.player.clone(),
        transform: Transform::from_translation(Vec3::new(-220.0, 0.0, 50.0)),
        sprite: Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(74.0, 60.0)),
            ..default()
        },
        ..default()
    },));

    entity.insert((
        Player,
        LocalControlled,
        TeamMarker(Team::Player),
        InGameEntity,
        Name::new("Player"),
    ));
    entity.insert((
        Health {
            current: max_hp,
            max: max_hp,
        },
        Energy {
            current: 0.0,
            max: energy_max,
        },
        Gold(0),
        Combo::new(1.8),
        SkillSlots::default(),
        PlayerSkillState::default(),
        PlayerDriveInput::default(),
        Velocity::default(),
        MoveSpeed(move_speed),
        AttackPower(attack_power),
        FacingDirection(Vec2::X),
        AnimationState::Idle,
        CritChance(crit),
        RewardModifiers::default(),
        PlayerAnim {
            state: AnimationState::Idle,
            timer: Timer::from_seconds(0.12, TimerMode::Once),
        },
    ));
    entity.insert((AugmentInventory::default(), PlayerLevel::default()));
    entity.insert((
        AttackCooldown::new(attack_cd),
        RangedCooldown::new(ranged_cd),
        RangedRapidFire {
            ramp: 0,
            decay: Timer::from_seconds(0.65, TimerMode::Once),
        },
        DashCooldown::new(dash_cd),
        Skill1Cooldown {
            timer: Timer::from_seconds(skill1_cd, TimerMode::Once),
        },
        InvincibilityTimer {
            timer: Timer::from_seconds(inv_s, TimerMode::Once),
        },
        DashState::inactive(dash_speed, dash_duration),
        Hurtbox {
            team: Team::Player,
            size: Vec2::splat(30.0),
        },
        Flash::new(0.0),
        Knockback(Vec2::ZERO),
    ));
}

pub fn push_local_input_to_players(
    input: Res<PlayerInputState>,
    mut player_q: Query<
        &mut PlayerDriveInput,
        (With<Player>, With<LocalControlled>, Without<Replicated>),
    >,
) {
    for mut drive in &mut player_q {
        *drive = PlayerDriveInput {
            move_axis: input.move_axis,
            aim_world: input.aim_world,
            attack_pressed: input.attack_pressed,
            attack_held: input.attack_held,
            ranged_pressed: input.ranged_pressed,
            ranged_held: input.ranged_held,
            dash_pressed: input.dash_pressed,
            skill_1_pressed: input.skill_1_pressed,
            skill_2_pressed: input.skill_2_pressed,
            skill_3_pressed: input.skill_3_pressed,
            skill_4_pressed: input.skill_4_pressed,
            interact_pressed: input.interact_pressed,
            pause_pressed: input.pause_pressed,
            shop_pressed: input.shop_pressed,
            menu_confirm_pressed: input.attack_pressed || input.interact_pressed,
            menu_cancel_pressed: input.pause_pressed,
        };
    }
}

pub fn player_move_system(
    time: Res<Time>,
    room_state: Res<RoomState>,
    session_q: Query<&CoopSessionState, With<CoopSessionEntity>>,
    mut q: Query<
        (
            &PlayerDriveInput,
            &DashState,
            &MoveSpeed,
            Option<&DashResetSpeedBuff>,
            &mut Velocity,
            &mut Transform,
            Option<&GhostState>,
        ),
        (With<Player>, Without<Replicated>),
    >,
) {
    let coop_phase = session_q.get_single().ok().map(|session| session.phase);
    if matches!(*room_state, RoomState::BossFight) {
        // still movable
    }
    for (input, dash, move_speed, dash_reset_buff, mut vel, mut tf, ghost) in &mut q {
        if coop_phase.is_some_and(coop_phase_blocks_player_movement) {
            vel.0 = Vec2::ZERO;
            continue;
        }
        let move_scale = if matches!(ghost, Some(GhostState::Ghost)) {
            0.85
        } else {
            1.0
        };
        if dash.active {
            vel.0 = dash.dir * dash.speed;
        } else {
            let dash_reset_mult = dash_reset_buff
                .map(|buff| buff.move_speed_mult.max(1.0))
                .unwrap_or(1.0);
            vel.0 = input.move_axis * move_speed.0.max(0.0) * move_scale * dash_reset_mult;
        }
        vel.0 = clamp_length(vel.0, dash.speed.max(move_speed.0));
        tf.translation += (vel.0 * time.delta_seconds()).extend(0.0);

        let clamped = clamp_in_room(
            tf.translation.truncate(),
            Vec2::new(ROOM_HALF_WIDTH, ROOM_HALF_HEIGHT),
            28.0,
        );
        tf.translation.x = clamped.x;
        tf.translation.y = clamped.y;
    }
}

fn coop_phase_blocks_player_movement(phase: CoopPhase) -> bool {
    matches!(
        phase,
        CoopPhase::Paused
            | CoopPhase::Reward
            | CoopPhase::Rps
            | CoopPhase::Shop
            | CoopPhase::MatchOver
    )
}

pub fn player_facing_system(
    mut q: Query<
        (
            &PlayerDriveInput,
            &GlobalTransform,
            &mut FacingDirection,
            &Velocity,
        ),
        (With<Player>, Without<Replicated>),
    >,
) {
    for (input, tf, mut facing, vel) in &mut q {
        if let Some(world) = input.aim_world {
            let dir = (world - tf.translation().truncate()).try_normalize();
            if let Some(dir) = dir {
                facing.0 = dir;
                continue;
            }
        }
        if vel.0.length_squared() > 1.0
            && let Some(dir) = vel.0.try_normalize()
        {
            facing.0 = dir;
        }
    }
}

pub fn player_invincibility_system(
    time: Res<Time>,
    mut q: Query<&mut InvincibilityTimer, (With<Player>, Without<Replicated>)>,
) {
    for mut inv in &mut q {
        inv.timer.tick(time.delta());
    }
}

pub fn player_death_system(
    mut death_events: EventReader<DeathEvent>,
    mut player_q: Query<
        (Entity, &mut Health, &mut InvincibilityTimer),
        (With<Player>, Without<Replicated>),
    >,
    mut next_state: ResMut<NextState<GamePhase>>,
    test_mode: Res<TestMode>,
) {
    for ev in death_events.read() {
        let is_player = player_q.iter().any(|(e, _, _)| e == ev.entity);
        if is_player {
            // 临时测试模式：满血复活 + 2 秒无敌，跳过 GameOver（见 docs/test_mode_temp.md）
            if test_mode.0 {
                if let Ok((_, mut health, mut inv)) = player_q.get_mut(ev.entity) {
                    health.current = health.max;
                    inv.timer = Timer::from_seconds(2.0, TimerMode::Once);
                    inv.timer.reset();
                }
                return;
            }
            if evaluate_death(SessionMode::Solo, 0) == DeathDecision::GameOver {
                next_state.set(GamePhase::GameOver);
            }
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coop_door_choice_does_not_block_movement() {
        assert!(!coop_phase_blocks_player_movement(CoopPhase::DoorChoice));
    }

    #[test]
    fn coop_reward_still_blocks_movement() {
        assert!(coop_phase_blocks_player_movement(CoopPhase::Reward));
    }
}
