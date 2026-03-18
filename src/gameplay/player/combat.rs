use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::core::input::PlayerInputState;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::combat::components::{Hitbox, Lifetime, Team};
use crate::gameplay::combat::projectiles;
use crate::gameplay::effects::particles;
use crate::gameplay::map::InGameEntity;

use super::components::*;

pub fn player_attack_input_system(
    mut commands: Commands,
    input: Res<PlayerInputState>,
    time: Res<Time>,
    assets: Res<GameAssets>,
    data: Option<Res<GameDataRegistry>>,
    mut q: Query<
        (
            Entity,
            &GlobalTransform,
            &FacingDirection,
            &AttackPower,
            &mut AttackCooldown,
            &CritChance,
            &RewardModifiers,
        ),
        With<Player>,
    >,
) {
    let Ok((player_e, player_tf, facing, power, mut cd, crit, mods)) = q.get_single_mut() else {
        return;
    };
    cd.timer.tick(time.delta());
    if !input.attack_held || !cd.timer.finished() {
        return;
    }

    cd.timer.reset();

    spawn_player_melee_hitbox(
        &mut commands,
        &assets,
        player_e,
        player_tf,
        facing.0,
        power.0,
        crit.0,
    );

    if mods.bonus_projectile {
        let proj_speed = data
            .as_deref()
            .map(|d| d.player.move_speed)
            .unwrap_or(260.0)
            * 2.0;
        projectiles::spawn_player_projectile(
            &mut commands,
            &assets,
            player_e,
            player_tf.translation().truncate() + facing.0 * 18.0,
            facing.0 * proj_speed,
            power.0 * 0.65,
            crit.0,
        );
    }

    particles::spawn_hit_particles(
        &mut commands,
        &assets,
        player_tf.translation().truncate() + facing.0 * 20.0,
        Color::srgba(0.7, 1.0, 0.7, 0.9),
    );
}

pub fn player_ranged_input_system(
    mut commands: Commands,
    input: Res<PlayerInputState>,
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut q: Query<
        (
            Entity,
            &GlobalTransform,
            &FacingDirection,
            &AttackPower,
            &CritChance,
            &mut RangedCooldown,
        ),
        With<Player>,
    >,
) {
    let Ok((player_e, tf, facing, power, crit, mut cd)) = q.get_single_mut() else {
        return;
    };
    cd.timer.tick(time.delta());
    if !input.ranged_held || !cd.timer.finished() {
        return;
    }

    let cost = data
        .as_deref()
        .map(|d| d.player.ranged_energy_cost)
        .unwrap_or(12.0);
    if energy.current < cost {
        return;
    }
    energy.current = (energy.current - cost).max(0.0);

    // 连发加速：连续射击提升射速，松开后衰减。
    let cfg = data.as_deref().map(|d| &d.player);
    let base_cd = cfg.map(|c| c.ranged_base_cooldown_s).unwrap_or(0.45);
    let min_cd = cfg.map(|c| c.ranged_min_cooldown_s).unwrap_or(0.18);
    let max_ramp = cfg.map(|c| c.ranged_ramp_max).unwrap_or(8).max(1);
    rapid.ramp = (rapid.ramp + 1).min(max_ramp);
    let ramp_t = (rapid.ramp.saturating_sub(1) as f32) / (max_ramp.saturating_sub(1) as f32).max(1.0);
    let cd_s = (base_cd + (min_cd - base_cd) * ramp_t).max(min_cd);
    cd.timer.set_duration(std::time::Duration::from_secs_f32(cd_s));

    cd.timer.reset();

    let dir = facing.0;
    let speed = 720.0;
    projectiles::spawn_player_projectile(
        &mut commands,
        &assets,
        player_e,
        tf.translation().truncate() + dir * 18.0,
        dir * speed,
        power.0 * 0.6,
        crit.0,
    );
    particles::spawn_hit_particles(
        &mut commands,
        &assets,
        tf.translation().truncate() + dir * 20.0,
        Color::srgba(0.4, 0.85, 1.0, 0.9),
    );
}

pub fn spawn_player_melee_hitbox(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    owner_tf: &GlobalTransform,
    dir: Vec2,
    damage: f32,
    _crit: f32,
) {
    let pos = owner_tf.translation().truncate() + dir * 32.0;
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(60.0)),
            sprite: Sprite {
                color: Color::srgba(0.95, 0.98, 0.85, 0.35),
                custom_size: Some(Vec2::new(46.0, 26.0)),
                ..default()
            },
            ..default()
        },
        Hitbox {
            owner: Some(owner),
            team: Team::Player,
            size: Vec2::new(46.0, 26.0),
            damage,
            knockback: 360.0,
            can_crit: true,
            crit_chance: _crit,
            crit_multiplier: 1.75,
        },
        Lifetime(Timer::from_seconds(0.08, TimerMode::Once)),
        InGameEntity,
        Name::new("PlayerHitbox"),
    ));
}

pub fn update_attack_cooldowns(time: Res<Time>, mut q: Query<&mut AttackCooldown, With<Player>>) {
    let Ok(mut cd) = q.get_single_mut() else {
        return;
    };
    cd.timer.tick(time.delta());
}
