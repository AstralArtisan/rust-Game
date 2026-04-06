use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::coop::components::{
    CoopMeleeFlashState, CoopPhase, CoopSessionEntity, CoopSessionState, GhostState,
};
use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::combat::components::{
    ArcHitbox, DamageKind, Hitbox, Lifetime, Projectile, Team,
};
use crate::gameplay::combat::projectiles::{self, HitTargets, PierceCount};
use crate::gameplay::effects::particles;
use crate::gameplay::map::InGameEntity;
use crate::utils::entity::safe_despawn_recursive;

use super::components::*;

const BASE_RANGED_PROJECTILE_SPEED: f32 = 720.0;
const TRIPLE_SPREAD_ANGLE: f32 = 0.24;
const NOVA_PROJECTILE_COUNT: usize = 8;
const RANGED_BURST_DELAY_S: f32 = 0.06;
const EXTRA_PROJECTILE_SPREAD_ANGLE: f32 = 0.15;
const EXTRA_PROJECTILE_DAMAGE_MULT: f32 = 0.60;
const MELEE_HITBOX_LIFETIME_S: f32 = 0.09;
const MELEE_SLASH_EFFECT_LIFETIME_S: f32 = 0.18;
const SLASH_FRAME_COUNT: usize = 9;
const SWORD_WAVE_TRAVEL_DISTANCE: f32 = 160.0;
const SWORD_WAVE_SPEED: f32 = 620.0;
const SWORD_WAVE_LIFETIME_S: f32 = SWORD_WAVE_TRAVEL_DISTANCE / SWORD_WAVE_SPEED;

#[derive(Debug, Clone, Copy)]
pub(crate) struct MeleeSwingProfile {
    pub(crate) reach: f32,
    pub(crate) center_offset: f32,
    pub(crate) hitbox_size: Vec2,
    pub(crate) slash_size: Vec2,
}

#[derive(Component, Debug, Clone)]
pub struct MeleeSlashEffect {
    pub timer: Timer,
    pub base_alpha: f32,
    pub base_scale: Vec3,
    pub frame_count: usize,
}

#[derive(Component, Debug, Clone)]
pub struct DelayedRangedShot {
    pub timer: Timer,
    pub owner: Entity,
    pub pos: Vec2,
    pub dir: Vec2,
    pub projectile_speed: f32,
    pub damage: f32,
    pub crit_chance: f32,
    pub pierce_remaining: u8,
}

pub fn player_attack_input_system(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut sfx_events: EventWriter<crate::core::events::SfxEvent>,
    session_q: Query<&CoopSessionState, With<CoopSessionEntity>>,
    mut q: Query<
        (
            Entity,
            &PlayerDriveInput,
            &GlobalTransform,
            &FacingDirection,
            &AttackPower,
            &mut AttackCooldown,
            &CritChance,
            &RewardModifiers,
            &Combo,
            &DashState,
            Option<&AugmentInventory>,
            Option<&PlayerSkillState>,
            Option<&GhostState>,
            Option<&mut CoopMeleeFlashState>,
        ),
        (With<Player>, Without<Replicated>),
    >,
) {
    let phase = session_q
        .get_single()
        .map(|session| session.phase)
        .unwrap_or(CoopPhase::None);
    for (
        player_e,
        input,
        player_tf,
        facing,
        power,
        mut cd,
        crit,
        mods,
        combo,
        dash,
        inventory,
        skill_state,
        ghost,
        melee_flash,
    ) in &mut q
    {
        if !input.attack_held
            || !cd.timer.finished()
            || phase != CoopPhase::None
            || dash.active
            || skill_state.is_some_and(PlayerSkillState::blocks_attacks)
            || matches!(ghost, Some(GhostState::Ghost))
        {
            continue;
        }

        let mut melee_speed_bonus = mods.total_melee_speed_bonus();
        let combo_accelerate_stacks = inventory
            .map(|value| value.stacks(AugmentId::ComboAccelerate))
            .unwrap_or(0);
        if combo_accelerate_stacks > 0 {
            let (combo_threshold, combo_bonus) = if combo_accelerate_stacks >= 2 {
                (3, 0.40)
            } else {
                (5, 0.25)
            };
            if combo.count >= combo_threshold {
                melee_speed_bonus += combo_bonus;
            }
        }

        cd.apply_speed_bonus(melee_speed_bonus);
        cd.timer.reset();
        sfx_events.send(crate::core::events::SfxEvent { kind: crate::core::events::SfxKind::MeleeAttack });
        let swing = melee_swing_profile(*mods);

        spawn_player_melee_hitbox_with_mods(
            &mut commands,
            &assets,
            player_e,
            player_tf,
            facing.0,
            power.0 * mods.melee_damage_mult(),
            crit.0,
            *mods,
            inventory,
        );

        particles::spawn_hit_particles(
            &mut commands,
            &assets,
            player_tf.translation().truncate() + facing.0 * (swing.reach - 8.0),
            Color::srgba(0.7, 1.0, 0.7, 0.9),
        );
        if let Some(mut melee_flash) = melee_flash {
            melee_flash.sequence = melee_flash.sequence.wrapping_add(1).max(1);
            melee_flash.slash_angle_rad = facing.0.y.atan2(facing.0.x);
        }
    }
}

pub fn player_ranged_input_system(
    mut commands: Commands,
    time: Res<Time>,
    data: Option<Res<GameDataRegistry>>,
    assets: Res<GameAssets>,
    mut sfx_events: EventWriter<crate::core::events::SfxEvent>,
    session_q: Query<&CoopSessionState, With<CoopSessionEntity>>,
    mut q: Query<
        (
            Entity,
            &PlayerDriveInput,
            &GlobalTransform,
            &FacingDirection,
            &AttackPower,
            &CritChance,
            &mut RangedCooldown,
            &mut RangedRapidFire,
            &RewardModifiers,
            &DashState,
            Option<&AugmentInventory>,
            Option<&PlayerSkillState>,
            Option<&GhostState>,
        ),
        (With<Player>, Without<Replicated>),
    >,
) {
    let phase = session_q
        .get_single()
        .map(|session| session.phase)
        .unwrap_or(CoopPhase::None);
    for (
        player_e,
        input,
        tf,
        facing,
        power,
        crit,
        mut cd,
        mut rapid,
        mods,
        dash,
        inventory,
        skill_state,
        ghost,
    ) in &mut q
    {
        if input.ranged_held {
            rapid.decay.reset();
            rapid.ramp = 1;
        } else {
            rapid.decay.tick(time.delta());
            if rapid.decay.finished() {
                rapid.ramp = 0;
            }
            continue;
        }

        if !cd.timer.finished()
            || phase != CoopPhase::None
            || dash.active
            || skill_state.is_some_and(PlayerSkillState::blocks_attacks)
            || matches!(ghost, Some(GhostState::Ghost))
        {
            continue;
        }

        let cfg = data.as_deref().map(|d| &d.player);
        cd.base_duration_s = cfg
            .map(|c| c.ranged_cooldown_s)
            .unwrap_or(cd.base_duration_s);
        cd.apply_speed_bonus(mods.total_ranged_speed_bonus());
        cd.timer.reset();
        sfx_events.send(crate::core::events::SfxEvent { kind: crate::core::events::SfxKind::RangedAttack });

        let dir = facing.0;
        let speed_boost_mult = match inventory
            .map(|value| value.stacks(AugmentId::SpeedBoost))
            .unwrap_or(0)
        {
            2 => 1.50,
            1 => 1.30,
            _ => 1.0,
        };
        let speed = BASE_RANGED_PROJECTILE_SPEED
            * mods.ranged_projectile_speed_mult()
            * speed_boost_mult;
        let damage = power.0 * 0.65 * mods.ranged_damage_mult();
        spawn_player_ranged_volley(
            &mut commands,
            &assets,
            player_e,
            tf.translation().truncate() + dir * 18.0,
            dir,
            speed,
            damage,
            crit.0,
            *mods,
            inventory,
        );
        particles::spawn_hit_particles(
            &mut commands,
            &assets,
            tf.translation().truncate() + dir * 20.0,
            Color::srgba(0.4, 0.85, 1.0, 0.9),
        );
    }
}

pub fn spawn_player_melee_hitbox(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    owner_tf: &GlobalTransform,
    dir: Vec2,
    damage: f32,
    crit_chance: f32,
) {
    spawn_player_melee_hitbox_with_mods(
        commands,
        assets,
        owner,
        owner_tf,
        dir,
        damage,
        crit_chance,
        RewardModifiers::default(),
        None,
    );
}

pub fn spawn_player_melee_hitbox_with_mods(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    owner_tf: &GlobalTransform,
    dir: Vec2,
    damage: f32,
    crit_chance: f32,
    mods: RewardModifiers,
    inventory: Option<&AugmentInventory>,
) {
    let owner_pos = owner_tf.translation().truncate();
    let direction = dir.try_normalize().unwrap_or(Vec2::X);
    let swing = melee_swing_profile(mods);
    let half_angle = mods.melee_arc_half_angle_rad();
    let pos = owner_pos + direction * swing.center_offset;
    let heavy_strike_stacks = inventory
        .map(|value| value.stacks(AugmentId::HeavyStrike))
        .unwrap_or(0);
    let (heavy_damage_mult, heavy_knockback_mult) = match heavy_strike_stacks {
        2 => (1.25, 2.20),
        1 => (1.15, 1.80),
        _ => (1.0, 1.0),
    };

    let slash_rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));
    let primary_color = if mods.melee_mastery_stacks >= 2 {
        Color::srgba(0.92, 1.0, 0.96, 0.90)
    } else {
        Color::srgba(0.84, 0.98, 0.96, 0.84)
    };
    spawn_melee_slash_visual(
        commands,
        assets,
        pos,
        slash_rotation,
        swing.slash_size,
        primary_color,
        61.0,
        Vec3::ONE,
        0.90,
    );

    if mods.melee_mastery_stacks >= 4 {
        spawn_melee_slash_visual(
            commands,
            assets,
            pos - direction * 10.0,
            slash_rotation,
            swing.slash_size * Vec2::new(1.05, 0.92),
            Color::srgba(0.52, 0.92, 1.0, 0.52),
            60.5,
            Vec3::splat(1.04),
            0.58,
        );
    }

    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(60.0)),
            sprite: Sprite {
                color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                custom_size: Some(swing.hitbox_size),
                ..default()
            },
            ..default()
        },
        Hitbox {
            owner: Some(owner),
            team: Team::Player,
            damage_kind: DamageKind::PlayerMelee,
            size: swing.hitbox_size,
            damage: damage * heavy_damage_mult,
            knockback: (360.0 + mods.melee_mastery_stacks as f32 * 12.0) * heavy_knockback_mult,
            can_crit: true,
            crit_chance,
            crit_multiplier: 1.75,
        },
        ArcHitbox {
            origin: owner_pos,
            direction,
            radius: swing.reach,
            half_angle_rad: half_angle,
        },
        Lifetime(Timer::from_seconds(
            MELEE_HITBOX_LIFETIME_S,
            TimerMode::Once,
        )),
        InGameEntity,
        Name::new("PlayerHitbox"),
    ));

    if mods.melee_sword_wave_unlocked() {
        spawn_player_sword_wave(
            commands,
            assets,
            owner,
            owner_pos + direction * (swing.reach + 12.0),
            direction,
            damage * mods.melee_sword_wave_damage_fraction(),
        );
    }
}

pub fn spawn_player_ranged_volley(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    dir: Vec2,
    projectile_speed: f32,
    damage: f32,
    crit_chance: f32,
    mods: RewardModifiers,
    inventory: Option<&AugmentInventory>,
) {
    let burst_count = if mods.ranged_mastery_stacks >= 2 {
        2
    } else {
        1
    };
    for burst_index in 0..burst_count {
        let delay_s = burst_index as f32 * RANGED_BURST_DELAY_S;
        spawn_ranged_burst(
            commands,
            assets,
            owner,
            pos,
            dir,
            projectile_speed,
            damage,
            crit_chance,
            mods.ranged_volley_pattern(),
            delay_s,
            inventory,
        );
    }
}

fn spawn_ranged_burst(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    dir: Vec2,
    projectile_speed: f32,
    damage: f32,
    crit_chance: f32,
    pattern: RangedVolleyPattern,
    delay_s: f32,
    inventory: Option<&AugmentInventory>,
) {
    let extra_projectiles = match inventory
        .map(|value| value.stacks(AugmentId::ExtraProjectile))
        .unwrap_or(0)
    {
        2 => 2,
        1 => 1,
        _ => 0,
    };
    let pierce_remaining = match inventory
        .map(|value| value.stacks(AugmentId::Piercing))
        .unwrap_or(0)
    {
        2 => 2,
        1 => 1,
        _ => 0,
    };

    match pattern {
        RangedVolleyPattern::Single | RangedVolleyPattern::Double => {
            queue_or_spawn_ranged_projectile(
                commands,
                assets,
                owner,
                pos,
                dir,
                projectile_speed,
                damage
                    * if matches!(pattern, RangedVolleyPattern::Double) {
                        0.62
                    } else {
                        1.0
                    },
                crit_chance,
                delay_s,
                pierce_remaining,
            );
            spawn_extra_projectiles_for_burst(
                commands,
                assets,
                owner,
                pos,
                dir,
                projectile_speed,
                damage,
                crit_chance,
                delay_s,
                extra_projectiles,
                pierce_remaining,
            );
        }
        RangedVolleyPattern::Triple => {
            for angle in [-TRIPLE_SPREAD_ANGLE, 0.0, TRIPLE_SPREAD_ANGLE] {
                let shot_dir = Mat2::from_angle(angle).mul_vec2(dir);
                let shot_damage = if angle == 0.0 {
                    damage * 0.52
                } else {
                    damage * 0.34
                };
                queue_or_spawn_ranged_projectile(
                    commands,
                    assets,
                    owner,
                    pos,
                    shot_dir,
                    projectile_speed,
                    shot_damage,
                    crit_chance,
                    delay_s,
                    pierce_remaining,
                );
            }
            spawn_extra_projectiles_for_burst(
                commands,
                assets,
                owner,
                pos,
                dir,
                projectile_speed,
                damage,
                crit_chance,
                delay_s,
                extra_projectiles,
                pierce_remaining,
            );
        }
        RangedVolleyPattern::Nova => {
            queue_or_spawn_ranged_projectile(
                commands,
                assets,
                owner,
                pos,
                dir,
                projectile_speed,
                damage * 0.48,
                crit_chance,
                delay_s,
                pierce_remaining,
            );

            let base_angle = dir.y.atan2(dir.x);
            for i in 0..NOVA_PROJECTILE_COUNT {
                let angle =
                    base_angle + i as f32 / NOVA_PROJECTILE_COUNT as f32 * std::f32::consts::TAU;
                let shot_dir = Vec2::new(angle.cos(), angle.sin());
                queue_or_spawn_ranged_projectile(
                    commands,
                    assets,
                    owner,
                    pos,
                    shot_dir,
                    projectile_speed,
                    damage * 0.20,
                    crit_chance,
                    delay_s,
                    pierce_remaining,
                );
            }
            spawn_extra_projectiles_for_burst(
                commands,
                assets,
                owner,
                pos,
                dir,
                projectile_speed,
                damage,
                crit_chance,
                delay_s,
                extra_projectiles,
                pierce_remaining,
            );
        }
    }
}

fn spawn_extra_projectiles_for_burst(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    dir: Vec2,
    projectile_speed: f32,
    damage: f32,
    crit_chance: f32,
    delay_s: f32,
    extra_projectiles: u8,
    pierce_remaining: u8,
) {
    for extra_index in 0..extra_projectiles {
        let angle = if extra_projectiles == 1 || extra_index > 0 {
            EXTRA_PROJECTILE_SPREAD_ANGLE
        } else {
            -EXTRA_PROJECTILE_SPREAD_ANGLE
        };
        let shot_dir = Mat2::from_angle(angle).mul_vec2(dir);
        queue_or_spawn_ranged_projectile(
            commands,
            assets,
            owner,
            pos,
            shot_dir,
            projectile_speed,
            damage * EXTRA_PROJECTILE_DAMAGE_MULT,
            crit_chance,
            delay_s,
            pierce_remaining,
        );
    }
}

fn queue_or_spawn_ranged_projectile(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    dir: Vec2,
    projectile_speed: f32,
    damage: f32,
    crit_chance: f32,
    delay_s: f32,
    pierce_remaining: u8,
) {
    if delay_s <= 0.0 {
        spawn_ranged_projectile(
            commands,
            assets,
            owner,
            pos,
            dir,
            projectile_speed,
            damage,
            crit_chance,
            pierce_remaining,
        );
        return;
    }

    commands.spawn((
        DelayedRangedShot {
            timer: Timer::from_seconds(delay_s, TimerMode::Once),
            owner,
            pos,
            dir,
            projectile_speed,
            damage,
            crit_chance,
            pierce_remaining,
        },
        InGameEntity,
        Name::new("DelayedRangedShot"),
    ));
}

fn spawn_ranged_projectile(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    dir: Vec2,
    projectile_speed: f32,
    damage: f32,
    crit_chance: f32,
    pierce_remaining: u8,
) {
    let projectile = projectiles::spawn_player_projectile(
        commands,
        assets,
        owner,
        pos,
        dir * projectile_speed,
        damage,
        crit_chance,
    );
    if pierce_remaining > 0 {
        commands.entity(projectile).insert((
            PierceCount {
                remaining: pierce_remaining,
            },
            HitTargets::default(),
        ));
    }
}

pub fn update_attack_cooldowns(
    time: Res<Time>,
    mut q: Query<(&mut AttackCooldown, &mut RangedCooldown), (With<Player>, Without<Replicated>)>,
) {
    for (mut attack_cd, mut ranged_cd) in &mut q {
        attack_cd.timer.tick(time.delta());
        ranged_cd.timer.tick(time.delta());
    }
}

pub fn update_melee_slash_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(
        Entity,
        &mut MeleeSlashEffect,
        &mut Sprite,
        &mut Transform,
        Option<&mut TextureAtlas>,
    )>,
) {
    for (entity, mut effect, mut sprite, mut transform, atlas) in &mut q {
        effect.timer.tick(time.delta());
        let progress = effect.timer.fraction();
        sprite
            .color
            .set_alpha(effect.base_alpha * (1.0 - progress).clamp(0.0, 1.0));
        transform.scale = effect.base_scale * (1.0 + progress * 0.18);
        if let Some(mut atlas) = atlas {
            atlas.index = ((progress * effect.frame_count as f32).floor() as usize)
                .min(effect.frame_count.saturating_sub(1));
        }

        if effect.timer.finished() {
            safe_despawn_recursive(&mut commands, entity);
        }
    }
}

pub fn update_delayed_ranged_shots(
    mut commands: Commands,
    assets: Res<GameAssets>,
    time: Res<Time>,
    mut q: Query<(Entity, &mut DelayedRangedShot)>,
) {
    for (entity, mut shot) in &mut q {
        shot.timer.tick(time.delta());
        if !shot.timer.finished() {
            continue;
        }

        spawn_ranged_projectile(
            &mut commands,
            &assets,
            shot.owner,
            shot.pos,
            shot.dir,
            shot.projectile_speed,
            shot.damage,
            shot.crit_chance,
            shot.pierce_remaining,
        );
        safe_despawn_recursive(&mut commands, entity);
    }
}

pub(crate) fn melee_swing_profile(mods: RewardModifiers) -> MeleeSwingProfile {
    let reach = 68.0 + mods.melee_range_bonus() * 1.45;
    let center_offset = reach * 0.42;
    let slash_size = Vec2::new(
        reach * 1.22,
        (72.0 + mods.melee_mastery_stacks as f32 * 6.0) * mods.melee_slash_scale(),
    );
    let hitbox_size = Vec2::new(reach * 1.16, reach * 1.16);
    MeleeSwingProfile {
        reach,
        center_offset,
        hitbox_size,
        slash_size,
    }
}

pub(crate) fn spawn_melee_slash_visual(
    commands: &mut Commands,
    assets: &GameAssets,
    pos: Vec2,
    rotation: Quat,
    size: Vec2,
    color: Color,
    z: f32,
    base_scale: Vec3,
    base_alpha: f32,
) -> Entity {
    commands
        .spawn((
            SpriteBundle {
                texture: assets.textures.slash.clone(),
                transform: Transform {
                    translation: pos.extend(z),
                    rotation,
                    scale: base_scale,
                },
                sprite: Sprite {
                    color,
                    custom_size: Some(size),
                    ..default()
                },
                ..default()
            },
            TextureAtlas {
                layout: assets.textures.slash_layout.clone(),
                index: 0,
            },
            MeleeSlashEffect {
                timer: Timer::from_seconds(MELEE_SLASH_EFFECT_LIFETIME_S, TimerMode::Once),
                base_alpha,
                base_scale,
                frame_count: SLASH_FRAME_COUNT,
            },
            InGameEntity,
            Name::new("MeleeSlashEffect"),
        ))
        .id()
}

fn spawn_player_sword_wave(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    dir: Vec2,
    damage: f32,
) {
    let direction = dir.try_normalize().unwrap_or(Vec2::X);
    let size = Vec2::new(82.0, 36.0);
    let velocity = direction * SWORD_WAVE_SPEED;
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.slash.clone(),
            transform: Transform {
                translation: pos.extend(59.0),
                rotation: Quat::from_rotation_z(direction.y.atan2(direction.x)),
                scale: Vec3::new(1.08, 0.72, 1.0),
            },
            sprite: Sprite {
                color: Color::srgba(0.64, 0.96, 1.0, 0.72),
                custom_size: Some(size),
                ..default()
            },
            ..default()
        },
        TextureAtlas {
            layout: assets.textures.slash_layout.clone(),
            index: 0,
        },
        Projectile {
            team: Team::Player,
            velocity,
        },
        Hitbox {
            owner: Some(owner),
            team: Team::Player,
            damage_kind: DamageKind::PlayerMelee,
            size: Vec2::new(56.0, 22.0),
            damage,
            knockback: 180.0,
            can_crit: false,
            crit_chance: 0.0,
            crit_multiplier: 1.0,
        },
        Lifetime(Timer::from_seconds(SWORD_WAVE_LIFETIME_S, TimerMode::Once)),
        MeleeSlashEffect {
            timer: Timer::from_seconds(SWORD_WAVE_LIFETIME_S, TimerMode::Once),
            base_alpha: 0.72,
            base_scale: Vec3::new(1.08, 0.72, 1.0),
            frame_count: SLASH_FRAME_COUNT,
        },
        InGameEntity,
        Name::new("SwordWave"),
    ));
}
