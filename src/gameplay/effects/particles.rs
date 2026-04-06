use bevy::prelude::*;
use rand::Rng;

use crate::core::assets::GameAssets;
use crate::gameplay::map::InGameEntity;
use crate::utils::entity::safe_despawn_recursive;

#[derive(Component, Debug, Clone)]
pub struct Particle {
    pub velocity: Vec2,
    pub lifetime: Timer,
}

pub fn spawn_hit_particles(commands: &mut Commands, assets: &GameAssets, pos: Vec2, color: Color) {
    spawn_hit_particles_count(commands, assets, pos, color, 6);
}

pub fn spawn_hit_particles_count(
    commands: &mut Commands,
    assets: &GameAssets,
    pos: Vec2,
    color: Color,
    count: u32,
) {
    let mut rng = rand::thread_rng();
    for i in 0..count {
        let angle = i as f32 / count as f32 * std::f32::consts::TAU + rng.gen_range(-0.3..0.3);
        let speed = rng.gen_range(120.0..200.0);
        let size = rng.gen_range(4.0..8.0);
        let vel = Vec2::new(angle.cos(), angle.sin()) * speed;

        // Main particle
        commands.spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(pos.extend(50.0)),
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::splat(size)),
                    ..default()
                },
                ..default()
            },
            Particle {
                velocity: vel,
                lifetime: Timer::from_seconds(0.25, TimerMode::Once),
            },
            InGameEntity,
            Name::new("HitParticle"),
        ));

        // Glow layer (larger, more transparent)
        commands.spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(pos.extend(49.0)),
                sprite: Sprite {
                    color: color.with_alpha(0.25),
                    custom_size: Some(Vec2::splat(size * 2.0)),
                    ..default()
                },
                ..default()
            },
            Particle {
                velocity: vel * 0.8,
                lifetime: Timer::from_seconds(0.2, TimerMode::Once),
            },
            InGameEntity,
            Name::new("HitParticleGlow"),
        ));
    }
}

pub fn spawn_dash_particles(commands: &mut Commands, assets: &GameAssets, pos: Vec2) {
    spawn_hit_particles(commands, assets, pos, Color::srgba(0.8, 0.9, 1.0, 0.7));
}

pub fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    for (e, mut p, mut tf, mut sprite) in &mut q {
        p.lifetime.tick(time.delta());
        tf.translation += (p.velocity * time.delta_seconds()).extend(0.0);
        // Decelerate
        p.velocity *= 1.0 - 3.0 * time.delta_seconds();
        sprite.color.set_alpha(1.0 - p.lifetime.fraction());
        if p.lifetime.finished() {
            safe_despawn_recursive(&mut commands, e);
        }
    }
}

// --- Expanding ring for visual effects ---

#[derive(Component, Debug, Clone)]
pub struct ExpandingRing {
    pub timer: Timer,
    pub initial_scale: f32,
    pub target_scale: f32,
}

pub fn update_expanding_rings(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut ExpandingRing, &mut Transform, &mut Sprite)>,
) {
    for (e, mut ring, mut tf, mut sprite) in &mut q {
        ring.timer.tick(time.delta());
        let t = ring.timer.fraction();
        let scale = ring.initial_scale + (ring.target_scale - ring.initial_scale) * t;
        tf.scale = Vec3::splat(scale);
        sprite.color.set_alpha((1.0 - t).clamp(0.0, 1.0) * 0.6);
        if ring.timer.finished() {
            safe_despawn_recursive(&mut commands, e);
        }
    }
}

// --- Whirlwind visual ---

pub fn spawn_whirlwind_visual(commands: &mut Commands, assets: &GameAssets, pos: Vec2) {
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(55.0)),
            sprite: Sprite {
                color: Color::srgba(0.9, 0.95, 1.0, 0.5),
                custom_size: Some(Vec2::new(80.0, 80.0)),
                ..default()
            },
            ..default()
        },
        ExpandingRing {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
            initial_scale: 0.3,
            target_scale: 1.2,
        },
        InGameEntity,
        Name::new("WhirlwindVisual"),
    ));
}

// --- Chain lightning visual ---

pub fn spawn_lightning_segment(commands: &mut Commands, assets: &GameAssets, from: Vec2, to: Vec2) {
    let _mid = (from + to) * 0.5;
    let diff = to - from;
    let len = diff.length();
    let angle = diff.y.atan2(diff.x);
    let mut rng = rand::thread_rng();
    let segments = 3;
    for i in 0..segments {
        let t = (i as f32 + 0.5) / segments as f32;
        let offset = Vec2::new(rng.gen_range(-4.0..4.0), rng.gen_range(-4.0..4.0));
        let seg_pos = from + diff * t + offset;
        commands.spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform {
                    translation: seg_pos.extend(55.0),
                    rotation: Quat::from_rotation_z(angle),
                    ..default()
                },
                sprite: Sprite {
                    color: Color::srgba(0.6, 0.85, 1.0, 0.8),
                    custom_size: Some(Vec2::new(len / segments as f32, 3.0)),
                    ..default()
                },
                ..default()
            },
            Particle {
                velocity: Vec2::ZERO,
                lifetime: Timer::from_seconds(0.15, TimerMode::Once),
            },
            InGameEntity,
            Name::new("LightningSegment"),
        ));
    }
}

// --- Scatter fan visual ---

pub fn spawn_scatter_fan(commands: &mut Commands, assets: &GameAssets, pos: Vec2, dir: Vec2) {
    let angle = dir.y.atan2(dir.x);
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform {
                translation: (pos + dir * 20.0).extend(54.0),
                rotation: Quat::from_rotation_z(angle),
                ..default()
            },
            sprite: Sprite {
                color: Color::srgba(1.0, 0.92, 0.3, 0.35),
                custom_size: Some(Vec2::new(40.0, 30.0)),
                ..default()
            },
            ..default()
        },
        Particle {
            velocity: dir * 40.0,
            lifetime: Timer::from_seconds(0.1, TimerMode::Once),
        },
        InGameEntity,
        Name::new("ScatterFan"),
    ));
}

// --- Thorns visual ---

pub fn spawn_thorns_particles(commands: &mut Commands, assets: &GameAssets, pos: Vec2) {
    let mut rng = rand::thread_rng();
    for _ in 0..4 {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(100.0..180.0);
        let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
        commands.spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(pos.extend(52.0)),
                sprite: Sprite {
                    color: Color::srgba(1.0, 0.6, 0.2, 0.8),
                    custom_size: Some(Vec2::new(6.0, 3.0)),
                    ..default()
                },
                ..default()
            },
            Particle {
                velocity: vel,
                lifetime: Timer::from_seconds(0.2, TimerMode::Once),
            },
            InGameEntity,
            Name::new("ThornParticle"),
        ));
    }
}

// --- Blink visual ---

pub fn spawn_blink_particles(commands: &mut Commands, assets: &GameAssets, from: Vec2, to: Vec2) {
    let mut rng = rand::thread_rng();
    // Dispersal at origin
    for _ in 0..5 {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(80.0..160.0);
        commands.spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(from.extend(52.0)),
                sprite: Sprite {
                    color: Color::srgba(0.7, 0.3, 1.0, 0.7),
                    custom_size: Some(Vec2::splat(5.0)),
                    ..default()
                },
                ..default()
            },
            Particle {
                velocity: Vec2::new(angle.cos(), angle.sin()) * speed,
                lifetime: Timer::from_seconds(0.2, TimerMode::Once),
            },
            InGameEntity,
            Name::new("BlinkOut"),
        ));
    }
    // Convergence at destination
    for _ in 0..5 {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let offset = Vec2::new(angle.cos(), angle.sin()) * 30.0;
        commands.spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation((to + offset).extend(52.0)),
                sprite: Sprite {
                    color: Color::srgba(0.7, 0.3, 1.0, 0.7),
                    custom_size: Some(Vec2::splat(5.0)),
                    ..default()
                },
                ..default()
            },
            Particle {
                velocity: -offset.normalize_or_zero() * 120.0,
                lifetime: Timer::from_seconds(0.2, TimerMode::Once),
            },
            InGameEntity,
            Name::new("BlinkIn"),
        ));
    }
}

// --- Bullet storm burst visual ---

pub fn spawn_burst_ring(commands: &mut Commands, assets: &GameAssets, pos: Vec2) {
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(54.0)),
            sprite: Sprite {
                color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                custom_size: Some(Vec2::splat(20.0)),
                ..default()
            },
            ..default()
        },
        ExpandingRing {
            timer: Timer::from_seconds(0.2, TimerMode::Once),
            initial_scale: 0.5,
            target_scale: 6.0,
        },
        InGameEntity,
        Name::new("BurstRing"),
    ));
}

// --- DashShield visual marker ---

#[derive(Component)]
pub struct ShieldVisual;
