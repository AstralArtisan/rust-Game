use bevy::prelude::*;
use rand::Rng;

use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
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
