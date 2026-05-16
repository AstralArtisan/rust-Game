use bevy::prelude::*;
use rand::Rng;

use crate::core::assets::GameAssets;
use crate::core::events::{DeathEvent, HitStopRequest, ScreenFlashRequest, SfxEvent, SfxKind};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::combat::components::Team;
use crate::gameplay::effects::particles::Particle;
use crate::gameplay::enemy::components::EnemyKind;
use crate::gameplay::enemy::components::EnemyType;
use crate::gameplay::map::InGameEntity;

pub fn death_effect_system(
    mut commands: Commands,
    mut death_events: EventReader<DeathEvent>,
    mut sfx_writer: EventWriter<SfxEvent>,
    mut hitstop_writer: EventWriter<HitStopRequest>,
    mut flash_writer: EventWriter<ScreenFlashRequest>,
    assets: Res<GameAssets>,
    registry: Res<GameDataRegistry>,
    kind_q: Query<&EnemyKind>,
    transform_q: Query<&Transform>,
    sprite_q: Query<&Sprite>,
) {
    let cfg = &registry.effects;
    let mut rng = rand::thread_rng();

    for ev in death_events.read() {
        if ev.team != Team::Enemy {
            continue;
        }

        let Ok(tf) = transform_q.get(ev.entity) else {
            continue;
        };
        let pos = tf.translation.truncate();

        let color = sprite_q
            .get(ev.entity)
            .map(|s| s.color)
            .unwrap_or(Color::srgba(1.0, 0.4, 0.3, 1.0));

        let is_boss = kind_q
            .get(ev.entity)
            .map(|k| k.0 == EnemyType::Boss)
            .unwrap_or(false);
        let particle_count = if is_boss {
            32
        } else {
            cfg.death_particle_count
        };

        // Spawn death particles
        for _ in 0..particle_count {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed = rng.gen_range(80.0..280.0);
            let size = rng.gen_range(2.5..6.0);
            let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
            let lifetime = if is_boss { 0.6 } else { 0.25 };

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
                    lifetime: Timer::from_seconds(lifetime, TimerMode::Once),
                },
                InGameEntity,
                Name::new("DeathParticle"),
            ));
        }

        // SFX
        if is_boss {
            sfx_writer.send(SfxEvent {
                kind: SfxKind::BossDeath,
            });
            hitstop_writer.send(HitStopRequest { duration_s: 0.12 });
            flash_writer.send(ScreenFlashRequest {
                color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                duration_s: 0.3,
            });
        } else {
            sfx_writer.send(SfxEvent {
                kind: SfxKind::EnemyDeath,
            });
            hitstop_writer.send(HitStopRequest {
                duration_s: cfg.hitstop_kill_s,
            });
        }
    }
}
