use bevy::prelude::*;

use crate::constants::{ROOM_HALF_HEIGHT, ROOM_HALF_WIDTH, UI_Z};
use crate::core::assets::GameAssets;
use crate::core::input::PlayerInputState;
use crate::gameplay::player::components::{Health, Velocity};
use crate::states::AppState;
use crate::ui::widgets;

use super::components::*;
use super::net::{
    NetMode, PvpFireMsg, PvpInputMsg, PvpNetConfig, PvpNetState, PvpPlayerStateMsg, PvpStateMsg,
};

#[derive(Resource, Debug, Default)]
pub struct PvpMatchState {
    pub tick: u32,
    pub state_send_timer: Timer,
}

impl PvpMatchState {
    fn ensure_init(&mut self) {
        if self.state_send_timer.duration().as_secs_f32() <= 0.0 {
            self.state_send_timer = Timer::from_seconds(1.0 / 30.0, TimerMode::Repeating);
        }
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct PvpOverlayState {
    pub pause_visible: bool,
}

#[derive(Component)]
pub struct PvpHudUi;

#[derive(Component)]
pub struct PvpHudText;

pub fn reset_pvp_runtime(
    mut commands: Commands,
    mut config: ResMut<PvpNetConfig>,
    mut net: ResMut<PvpNetState>,
    mut overlay: ResMut<PvpOverlayState>,
    q: Query<Entity, With<PvpEntity>>,
) {
    super::net::reset_pvp_network(&mut config, &mut net);
    overlay.pause_visible = false;
    for entity in &q {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn setup_pvp_game(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut match_state: ResMut<PvpMatchState>,
    mut net: ResMut<PvpNetState>,
    mut overlay: ResMut<PvpOverlayState>,
) {
    match_state.tick = 0;
    match_state.state_send_timer = Timer::from_seconds(1.0 / 30.0, TimerMode::Repeating);
    net.clear_runtime();
    overlay.pause_visible = false;

    // Arena backdrop.
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            sprite: Sprite {
                color: Color::srgb(0.06, 0.07, 0.10),
                custom_size: Some(Vec2::new(ROOM_HALF_WIDTH * 2.0, ROOM_HALF_HEIGHT * 2.0)),
                ..default()
            },
            ..default()
        },
        PvpEntity,
        Name::new("PvpArena"),
    ));

    // HUD.
    commands
        .spawn((
            widgets::root_node(),
            PvpHudUi,
            PvpEntity,
            Name::new("PvpHudRoot"),
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0),
                    top: Val::Px(12.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|col| {
                col.spawn((widgets::title_text(&assets, "PVP", 18.0), PvpHudText));
            });
        });

    commands
        .spawn((
            widgets::root_node(),
            PvpOverlayUi,
            PvpEntity,
            Name::new("PvpOverlayRoot"),
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(18.0),
                    top: Val::Px(72.0),
                    width: Val::Px(420.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    ..default()
                },
                visibility: Visibility::Hidden,
                background_color: BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.86)),
                ..default()
            })
            .with_children(|panel| {
                panel.spawn((widgets::body_text(&assets, "", 18.0), PvpOverlayText));
            });
        });

    spawn_players(&mut commands, &assets, net.my_id);
}

fn pvp_player_color(player_id: u8, flash: bool) -> Color {
    if flash {
        Color::srgb(1.0, 0.98, 0.92)
    } else if player_id == 1 {
        Color::srgb(0.25, 0.9, 1.0)
    } else {
        Color::srgb(1.0, 0.45, 0.30)
    }
}

fn spawn_players(commands: &mut Commands, assets: &GameAssets, my_id: Option<u8>) {
    let p1 = spawn_one_player(
        commands,
        assets,
        1,
        Vec2::new(-ROOM_HALF_WIDTH * 0.55, 0.0),
        pvp_player_color(1, false),
    );
    let p2 = spawn_one_player(
        commands,
        assets,
        2,
        Vec2::new(ROOM_HALF_WIDTH * 0.55, 0.0),
        pvp_player_color(2, false),
    );

    if my_id == Some(1) {
        commands.entity(p1).insert(PvpLocalPlayer);
        commands.entity(p2).insert(PvpRemotePlayer);
    } else if my_id == Some(2) {
        commands.entity(p2).insert(PvpLocalPlayer);
        commands.entity(p1).insert(PvpRemotePlayer);
    }
}

fn spawn_one_player(
    commands: &mut Commands,
    assets: &GameAssets,
    id: u8,
    pos: Vec2,
    color: Color,
) -> Entity {
    commands
        .spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(pos.extend(50.0)),
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::splat(34.0)),
                    ..default()
                },
                ..default()
            },
            PvpEntity,
            PvpPlayerId(id),
            Velocity::default(),
            Health {
                current: 100.0,
                max: 100.0,
            },
            PvpLives::default(),
            PvpCooldowns::new(),
            PvpNetTarget {
                pos,
                hp: 100.0,
                lives: 3,
            },
            PvpMeleeFlash::default(),
            Name::new(format!("PvpPlayer{id}")),
        ))
        .id()
}

pub fn cleanup_pvp_world(mut commands: Commands, q: Query<Entity, With<PvpEntity>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

pub fn pvp_send_local_input_system(
    input: Res<PlayerInputState>,
    config: Res<PvpNetConfig>,
    net: Res<PvpNetState>,
    overlay: Res<PvpOverlayState>,
    time: Res<Time>,
    mut send_timer: Local<Timer>,
    mut last_sent: Local<Option<PvpInputMsg>>,
) {
    if config.mode != NetMode::Client || !net.connected {
        *last_sent = None;
        return;
    }

    if send_timer.duration().as_secs_f32() <= 0.0 {
        *send_timer = Timer::from_seconds(1.0 / 30.0, TimerMode::Repeating);
    }
    send_timer.tick(time.delta());

    let msg = if overlay.pause_visible {
        PvpInputMsg::default()
    } else {
        let aim = input.aim_world.unwrap_or(Vec2::ZERO);
        PvpInputMsg {
            move_axis: (input.move_axis.x, input.move_axis.y),
            melee: input.attack_pressed,
            ranged: input.ranged_pressed,
            aim: (aim.x, aim.y),
        }
    };
    let changed = last_sent
        .as_ref()
        .map(|previous| *previous != msg)
        .unwrap_or(true);
    if !changed && !send_timer.just_finished() {
        return;
    }

    net.send_input(msg);
    *last_sent = Some(msg);
}

pub fn pvp_host_simulation_system(
    time: Res<Time>,
    input: Res<PlayerInputState>,
    mut match_state: ResMut<PvpMatchState>,
    config: Res<PvpNetConfig>,
    mut net: ResMut<PvpNetState>,
    overlay: Res<PvpOverlayState>,
    mut next: ResMut<NextState<AppState>>,
    mut players: Query<(
        &PvpPlayerId,
        &mut Transform,
        &mut Velocity,
        &mut Health,
        &mut PvpLives,
        &mut PvpCooldowns,
    )>,
) {
    if config.mode != NetMode::Host || !net.connected {
        return;
    }
    match_state.ensure_init();
    match_state.tick = match_state.tick.wrapping_add(1);

    let client_input = net.client_input();

    // Host input (player 1).
    let host_input = if overlay.pause_visible {
        PvpInputMsg::default()
    } else {
        let host_aim = input.aim_world.unwrap_or(Vec2::ZERO);
        PvpInputMsg {
            move_axis: (input.move_axis.x, input.move_axis.y),
            melee: input.attack_pressed,
            ranged: input.ranged_pressed,
            aim: (host_aim.x, host_aim.y),
        }
    };

    // Tick cooldowns & simulate movement.
    for (id, mut tf, mut vel, _hp, _lives, mut cds) in &mut players {
        cds.melee.tick(time.delta());
        cds.ranged.tick(time.delta());
        cds.respawn.tick(time.delta());

        if !cds.respawn.finished() {
            vel.0 = Vec2::ZERO;
            continue;
        }

        let axis = if id.0 == 1 {
            Vec2::new(host_input.move_axis.0, host_input.move_axis.1)
        } else {
            Vec2::new(client_input.move_axis.0, client_input.move_axis.1)
        };
        let speed = 310.0;
        vel.0 = axis * speed;
        tf.translation += (vel.0 * time.delta_seconds()).extend(0.0);
        clamp_to_arena(&mut tf);
    }

    // Resolve attacks.
    resolve_attacks(&mut players, host_input, client_input, &mut net, &mut next);

    // Send snapshot at 20hz.
    match_state.state_send_timer.tick(time.delta());
    if match_state.state_send_timer.just_finished() {
        if let Some(st) = build_state_msg(match_state.tick, &mut players) {
            net.send_state(&st);
        }
    }
}

fn resolve_attacks(
    players: &mut Query<(
        &PvpPlayerId,
        &mut Transform,
        &mut Velocity,
        &mut Health,
        &mut PvpLives,
        &mut PvpCooldowns,
    )>,
    host_input: PvpInputMsg,
    client_input: PvpInputMsg,
    net: &mut PvpNetState,
    next: &mut NextState<AppState>,
) {
    // Gather data.
    let mut p1 = None;
    let mut p2 = None;
    for (id, tf, _vel, hp, lives, cds) in players.iter_mut() {
        if id.0 == 1 {
            p1 = Some((
                tf.translation.truncate(),
                hp.current,
                lives.lives,
                cds.respawn.finished(),
            ));
        } else if id.0 == 2 {
            p2 = Some((
                tf.translation.truncate(),
                hp.current,
                lives.lives,
                cds.respawn.finished(),
            ));
        }
    }
    let (Some((p1_pos, _p1_hp, _p1_l, p1_alive)), Some((p2_pos, _p2_hp, _p2_l, p2_alive))) =
        (p1, p2)
    else {
        return;
    };

    // Melee: short range, higher damage.
    if host_input.melee && p1_alive {
        try_melee(1, 2, p1_pos, p2_pos, players);
    }
    if client_input.melee && p2_alive {
        try_melee(2, 1, p2_pos, p1_pos, players);
    }

    // Ranged: hitscan + bullet visual.
    if host_input.ranged && p1_alive {
        try_ranged(
            1,
            2,
            p1_pos,
            Vec2::new(host_input.aim.0, host_input.aim.1),
            p2_pos,
            players,
            net,
        );
    }
    if client_input.ranged && p2_alive {
        try_ranged(
            2,
            1,
            p2_pos,
            Vec2::new(client_input.aim.0, client_input.aim.1),
            p1_pos,
            players,
            net,
        );
    }

    // Death / respawn / win.
    let mut p1_lives = None;
    let mut p2_lives = None;
    let mut p1_hp = None;
    let mut p2_hp = None;
    for (id, _tf, _vel, hp, lives, _cds) in players.iter_mut() {
        if id.0 == 1 {
            p1_lives = Some(lives.lives);
            p1_hp = Some(hp.current);
        } else if id.0 == 2 {
            p2_lives = Some(lives.lives);
            p2_hp = Some(hp.current);
        }
    }
    let (Some(p1_l), Some(p2_l), Some(p1_h), Some(p2_h)) = (p1_lives, p2_lives, p1_hp, p2_hp)
    else {
        return;
    };
    if p1_h <= 0.0 {
        handle_death(1, players);
    }
    if p2_h <= 0.0 {
        handle_death(2, players);
    }

    // Winner check after deaths applied.
    let mut p1_left = 0;
    let mut p2_left = 0;
    for (id, _tf, _vel, _hp, lives, _cds) in players.iter_mut() {
        if id.0 == 1 {
            p1_left = lives.lives;
        } else if id.0 == 2 {
            p2_left = lives.lives;
        }
    }
    if p1_left == 0 || p2_left == 0 {
        let winner = if p1_left > 0 { 1 } else { 2 };
        net.send_result(winner);
        next.set(AppState::PvpResult);
    }
}

fn try_melee(
    attacker: u8,
    target: u8,
    attacker_pos: Vec2,
    target_pos: Vec2,
    players: &mut Query<(
        &PvpPlayerId,
        &mut Transform,
        &mut Velocity,
        &mut Health,
        &mut PvpLives,
        &mut PvpCooldowns,
    )>,
) {
    let range = 54.0;
    if attacker_pos.distance(target_pos) > range {
        return;
    }

    let mut can = false;
    for (id, _tf, _vel, _hp, _lives, cds) in players.iter_mut() {
        if id.0 == attacker && cds.melee.finished() && cds.respawn.finished() {
            can = true;
            break;
        }
    }
    if !can {
        return;
    }

    for (id, _tf, _vel, _hp, _lives, mut cds) in players.iter_mut() {
        if id.0 == attacker {
            cds.melee.reset();
        }
    }

    let dir = (target_pos - attacker_pos)
        .try_normalize()
        .unwrap_or(Vec2::X);
    for (id, _tf, mut vel, mut hp, _lives, _cds) in players.iter_mut() {
        if id.0 == target {
            hp.current = (hp.current - 18.0).max(0.0);
            vel.0 += dir * 420.0;
        }
    }
}

fn try_ranged(
    attacker: u8,
    target: u8,
    attacker_pos: Vec2,
    aim_world: Vec2,
    target_pos: Vec2,
    players: &mut Query<(
        &PvpPlayerId,
        &mut Transform,
        &mut Velocity,
        &mut Health,
        &mut PvpLives,
        &mut PvpCooldowns,
    )>,
    net: &mut PvpNetState,
) {
    let mut can = false;
    for (id, _tf, _vel, _hp, _lives, cds) in players.iter_mut() {
        if id.0 == attacker && cds.ranged.finished() && cds.respawn.finished() {
            can = true;
            break;
        }
    }
    if !can {
        return;
    }
    for (id, _tf, _vel, _hp, _lives, mut cds) in players.iter_mut() {
        if id.0 == attacker {
            cds.ranged.reset();
        }
    }

    let dir = (aim_world - attacker_pos)
        .try_normalize()
        .unwrap_or(Vec2::X);
    let fire = PvpFireMsg {
        shooter_id: attacker,
        origin: (attacker_pos.x, attacker_pos.y),
        dir: (dir.x, dir.y),
    };
    net.fire_events.push(fire);
    net.send_fire(fire);

    // Hitscan: if target is close to ray.
    let max_range = 560.0;
    let to_target = target_pos - attacker_pos;
    if to_target.length() > max_range {
        return;
    }
    let proj = to_target.dot(dir);
    if proj < 0.0 {
        return;
    }
    let closest = attacker_pos + dir * proj;
    let dist = closest.distance(target_pos);
    if dist > 22.0 {
        return;
    }

    for (id, _tf, mut vel, mut hp, _lives, _cds) in players.iter_mut() {
        if id.0 == target {
            hp.current = (hp.current - 10.0).max(0.0);
            vel.0 += dir * 280.0;
        }
    }
}

fn handle_death(
    who: u8,
    players: &mut Query<(
        &PvpPlayerId,
        &mut Transform,
        &mut Velocity,
        &mut Health,
        &mut PvpLives,
        &mut PvpCooldowns,
    )>,
) {
    let mut lives_left = 0;
    for (id, _tf, _vel, _hp, lives, _cds) in players.iter_mut() {
        if id.0 == who {
            lives_left = lives.lives;
        }
    }
    if lives_left == 0 {
        return;
    }

    for (id, mut tf, mut vel, mut hp, mut lives, mut cds) in players.iter_mut() {
        if id.0 != who {
            continue;
        }
        lives.lives = lives.lives.saturating_sub(1);
        vel.0 = Vec2::ZERO;
        hp.current = hp.max;
        cds.respawn = Timer::from_seconds(1.1, TimerMode::Once);
        cds.respawn.reset();

        let respawn_pos = if who == 1 {
            Vec2::new(-ROOM_HALF_WIDTH * 0.55, 0.0)
        } else {
            Vec2::new(ROOM_HALF_WIDTH * 0.55, 0.0)
        };
        tf.translation = respawn_pos.extend(tf.translation.z);
        clamp_to_arena(&mut tf);
    }
}

fn clamp_to_arena(tf: &mut Transform) {
    let half = Vec2::new(ROOM_HALF_WIDTH - 26.0, ROOM_HALF_HEIGHT - 26.0);
    tf.translation.x = tf.translation.x.clamp(-half.x, half.x);
    tf.translation.y = tf.translation.y.clamp(-half.y, half.y);
}

fn build_state_msg(
    tick: u32,
    players: &mut Query<(
        &PvpPlayerId,
        &mut Transform,
        &mut Velocity,
        &mut Health,
        &mut PvpLives,
        &mut PvpCooldowns,
    )>,
) -> Option<PvpStateMsg> {
    let mut p1 = None;
    let mut p2 = None;
    for (id, tf, _vel, hp, lives, cds) in players.iter_mut() {
        let msg = PvpPlayerStateMsg {
            id: id.0,
            pos: (tf.translation.x, tf.translation.y),
            hp: hp.current,
            lives: lives.lives,
            melee_flash: !cds.melee.finished() && cds.melee.elapsed_secs() < 0.12,
        };
        if id.0 == 1 {
            p1 = Some(msg);
        } else if id.0 == 2 {
            p2 = Some(msg);
        }
    }
    Some(PvpStateMsg {
        tick,
        p1: p1?,
        p2: p2?,
    })
}

pub fn pvp_client_apply_state_system(
    config: Res<PvpNetConfig>,
    mut net: ResMut<PvpNetState>,
    mut next: ResMut<NextState<AppState>>,
    mut players: Query<(
        &PvpPlayerId,
        &mut Transform,
        &mut Health,
        &mut PvpLives,
        &mut Velocity,
        &mut PvpCooldowns,
        &mut PvpNetTarget,
        Option<&PvpLocalPlayer>,
        &mut PvpMeleeFlash,
    )>,
) {
    if config.mode != NetMode::Client || !net.connected {
        return;
    }
    let Some(st) = net.last_state.take() else {
        return;
    };

    for (id, mut tf, mut hp, mut lives, mut vel, mut cds, mut target, local, mut flash) in &mut players {
        let src = if id.0 == 1 { st.p1 } else { st.p2 };
        let server_pos = Vec2::new(src.pos.0, src.pos.1);
        target.pos = server_pos;
        target.hp = src.hp;
        target.lives = src.lives;
        hp.current = src.hp;
        lives.lives = src.lives;
        if src.melee_flash {
            flash.timer = Timer::from_seconds(0.12, TimerMode::Once);
            flash.timer.reset();
        }
        if local.is_some() {
            let delta = tf.translation.truncate().distance(server_pos);
            if delta > 140.0 {
                tf.translation.x = server_pos.x;
                tf.translation.y = server_pos.y;
                vel.0 = Vec2::ZERO;
            }
        } else if tf.translation.truncate().distance(server_pos) > 220.0 {
            tf.translation.x = server_pos.x;
            tf.translation.y = server_pos.y;
            vel.0 = Vec2::ZERO;
        }
        cds.respawn = Timer::from_seconds(0.0, TimerMode::Once);
    }

    if let Some(w) = net.winner {
        let _ = w;
        next.set(AppState::PvpResult);
    }
}

pub fn pvp_client_local_prediction_system(
    time: Res<Time>,
    input: Res<PlayerInputState>,
    config: Res<PvpNetConfig>,
    overlay: Res<PvpOverlayState>,
    mut q: Query<(&mut Transform, &mut Velocity, &mut PvpCooldowns), With<PvpLocalPlayer>>,
) {
    if config.mode != NetMode::Client {
        return;
    }
    let Ok((mut tf, mut vel, mut cds)) = q.get_single_mut() else {
        return;
    };

    cds.melee.tick(time.delta());
    cds.ranged.tick(time.delta());
    if overlay.pause_visible || !cds.respawn.finished() {
        vel.0 = Vec2::ZERO;
        return;
    }

    let axis = input.move_axis;
    vel.0 = axis * 310.0;
    tf.translation += (vel.0 * time.delta_seconds()).extend(0.0);
    clamp_to_arena(&mut tf);

    if input.attack_pressed && cds.melee.finished() {
        cds.melee.reset();
    }
}

pub fn pvp_client_interpolate_players_system(
    time: Res<Time>,
    config: Res<PvpNetConfig>,
    mut q: Query<(
        &PvpNetTarget,
        &mut Transform,
        Option<&PvpLocalPlayer>,
        Option<&PvpRemotePlayer>,
    )>,
) {
    if config.mode != NetMode::Client {
        return;
    }

    let local_smooth = 1.0 - (-10.0 * time.delta_seconds()).exp();
    let remote_smooth = 1.0 - (-14.0 * time.delta_seconds()).exp();
    for (target, mut tf, local, remote) in &mut q {
        let current = tf.translation.truncate();
        let desired = target.pos;
        if local.is_some() {
            let next = if current.distance(desired) > 140.0 {
                desired
            } else {
                current.lerp(desired, local_smooth * 0.2)
            };
            tf.translation.x = next.x;
            tf.translation.y = next.y;
        } else if remote.is_some() {
            let next = if current.distance(desired) > 180.0 {
                desired
            } else {
                current.lerp(desired, remote_smooth)
            };
            tf.translation.x = next.x;
            tf.translation.y = next.y;
        }
    }
}

pub fn pvp_update_player_visuals_system(
    time: Res<Time>,
    mut q: Query<(&PvpPlayerId, &mut Sprite, &mut PvpMeleeFlash, &PvpCooldowns)>,
) {
    for (id, mut sprite, mut flash, cds) in &mut q {
        flash.timer.tick(time.delta());
        let is_flashing =
            !flash.timer.finished() || (!cds.melee.finished() && cds.melee.elapsed_secs() < 0.12);
        sprite.color = pvp_player_color(id.0, is_flashing);
    }
}

pub fn pvp_bullet_visual_system(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut net: ResMut<PvpNetState>,
) {
    if net.fire_events.is_empty() {
        return;
    }
    let events = std::mem::take(&mut net.fire_events);
    for ev in events {
        let origin = Vec2::new(ev.origin.0, ev.origin.1);
        let dir = Vec2::new(ev.dir.0, ev.dir.1)
            .try_normalize()
            .unwrap_or(Vec2::X);
        spawn_bullet_visual(&mut commands, &assets, ev.shooter_id, origin, dir);
    }
}

fn spawn_bullet_visual(
    commands: &mut Commands,
    assets: &GameAssets,
    shooter_id: u8,
    origin: Vec2,
    dir: Vec2,
) {
    let speed = 860.0;
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation((origin + dir * 22.0).extend(UI_Z - 20.0)),
            sprite: Sprite {
                color: pvp_player_color(shooter_id, false).with_alpha(0.92),
                custom_size: Some(Vec2::new(10.0, 4.0)),
                ..default()
            },
            ..default()
        },
        PvpEntity,
        PvpBullet {
            velocity: dir * speed,
        },
        crate::gameplay::combat::components::Lifetime(Timer::from_seconds(0.25, TimerMode::Once)),
        Name::new("PvpBullet"),
    ));
}

pub fn pvp_overlay_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<PvpOverlayState>,
    mut next: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        overlay.pause_visible = !overlay.pause_visible;
    }
    if !overlay.pause_visible {
        return;
    }
    if keyboard.just_pressed(KeyCode::KeyM) {
        next.set(AppState::MainMenu);
    } else if keyboard.just_pressed(KeyCode::KeyQ) {
        let _ = exit.send(AppExit::Success);
    }
}

pub fn update_pvp_overlay_ui_system(
    overlay: Res<PvpOverlayState>,
    mut root_q: Query<&mut Visibility, (With<PvpOverlayUi>, Without<PvpOverlayText>)>,
    mut text_q: Query<&mut Text, With<PvpOverlayText>>,
) {
    let Ok(mut visibility) = root_q.get_single_mut() else {
        return;
    };
    *visibility = if overlay.pause_visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    text.sections[0].value =
        "PVP 暂停\nESC：继续游戏\nM：回到主菜单\nQ：退出游戏".to_string();
}

pub fn pvp_bullet_visual_system_move_and_despawn(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(
        Entity,
        &PvpBullet,
        &mut Transform,
        &mut crate::gameplay::combat::components::Lifetime,
    )>,
) {
    for (e, bullet, mut tf, mut life) in &mut q {
        tf.translation += (bullet.velocity * time.delta_seconds()).extend(0.0);
        life.0.tick(time.delta());
        if life.0.finished() {
            commands.entity(e).despawn_recursive();
        }
    }
}

pub fn pvp_update_hud_system(
    net: Res<PvpNetState>,
    players: Query<(&PvpPlayerId, &Health, &PvpLives)>,
    mut text_q: Query<&mut Text, With<PvpHudText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    let mut p1 = None;
    let mut p2 = None;
    for (id, hp, lives) in &players {
        if id.0 == 1 {
            p1 = Some((hp.current, lives.lives));
        } else if id.0 == 2 {
            p2 = Some((hp.current, lives.lives));
        }
    }
    let (p1, p2) = match (p1, p2) {
        (Some(a), Some(b)) => (a, b),
        _ => return,
    };
    let me = net.my_id.unwrap_or(0);
    text.sections[0].value = format!(
        "PVP（你是P{me}）  P1: HP {:.0} / Lives {}    P2: HP {:.0} / Lives {}",
        p1.0, p1.1, p2.0, p2.1
    );
}
