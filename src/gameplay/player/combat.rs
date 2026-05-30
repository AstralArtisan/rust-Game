#![allow(dead_code)]

use bevy::prelude::*;
use lightyear::prelude::Replicated;
use std::f32::consts::PI;

use crate::coop::components::{
    CoopMeleeFlashState, CoopPhase, CoopSessionEntity, CoopSessionState, GhostState,
};
use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::augment::tuning;
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
const EXTRA_PROJECTILE_STAGGER_S: f32 = 0.08;
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
    pub crit_multiplier: f32,
    pub pierce_remaining: u8,
    pub homing_stacks: u8,
}

pub fn player_attack_input_system(
    mut commands: Commands,
    assets: Res<GameAssets>,
    data: Res<GameDataRegistry>,
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
            &Gold,
            Option<&AugmentInventory>,
            Option<&PlayerSkillState>,
            Option<&GhostState>,
            (Option<&PlayerBuff>, Option<&mut CoopMeleeFlashState>),
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
        gold,
        inventory,
        skill_state,
        ghost,
        (buff, melee_flash),
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
        // Buff attack_speed_bonus is a percentage of the current base cooldown;
        // converting it into a seconds-of-reduction lets it feed apply_speed_bonus
        // alongside the levelup-driven mods bonus.
        let buff_speed_pct = buff.map(|b| b.attack_speed_bonus).unwrap_or(0.0);
        if buff_speed_pct > 0.0 {
            melee_speed_bonus += buff_speed_pct * cd.base_duration_s;
        }
        let mut combo_crit_bonus = 0.0;
        let combo_accelerate_stacks = inventory
            .map(|value| value.stacks(AugmentId::ComboAccelerate))
            .unwrap_or(0);
        if combo_accelerate_stacks > 0 {
            let (combo_bonus, crit_bonus) =
                tuning::combo_accelerate_bonuses(&data, combo_accelerate_stacks, combo.count);
            melee_speed_bonus += combo_bonus;
            combo_crit_bonus += crit_bonus;
        }

        cd.apply_speed_bonus(melee_speed_bonus);
        // Clamp the resulting cooldown to the RON-configured floor; force_max
        // pins it directly to that floor.
        let melee_floor = data.rewards.levelup.melee_min_s.max(0.01);
        if buff.is_some_and(|b| b.force_attack_speed_max)
            || cd.timer.duration().as_secs_f32() < melee_floor
        {
            cd.timer
                .set_duration(std::time::Duration::from_secs_f32(melee_floor));
        }
        cd.timer.reset();
        sfx_events.send(crate::core::events::SfxEvent {
            kind: crate::core::events::SfxKind::MeleeAttack,
        });
        let swing = melee_swing_profile(*mods);

        let greed_mult = greed_damage_mult(&data, inventory, gold.0);
        let buff_attack = 1.0 + buff.map(|b| b.attack_bonus).unwrap_or(0.0);
        spawn_player_melee_hitbox_with_mods(
            &mut commands,
            &assets,
            &data,
            player_e,
            player_tf,
            facing.0,
            power.0 * mods.melee_damage_mult() * greed_mult * buff_attack,
            crit.0 + combo_crit_bonus + greed_crit_bonus(&data, inventory, gold.0),
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
            &Gold,
            Option<&AugmentInventory>,
            Option<&PlayerSkillState>,
            Option<&GhostState>,
            Option<&PlayerBuff>,
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
        gold,
        inventory,
        skill_state,
        ghost,
        buff,
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

        // base_duration_s is owned by the levelup system; do not reset per-shot.
        let mut ranged_speed_bonus = mods.total_ranged_speed_bonus();
        let buff_speed_pct = buff.map(|b| b.attack_speed_bonus).unwrap_or(0.0);
        if buff_speed_pct > 0.0 {
            ranged_speed_bonus += buff_speed_pct * cd.base_duration_s;
        }
        cd.apply_speed_bonus(ranged_speed_bonus);
        let ranged_floor = data
            .as_deref()
            .map(|d| d.rewards.levelup.ranged_min_s.max(0.01))
            .unwrap_or(0.12);
        if buff.is_some_and(|b| b.force_attack_speed_max)
            || cd.timer.duration().as_secs_f32() < ranged_floor
        {
            cd.timer
                .set_duration(std::time::Duration::from_secs_f32(ranged_floor));
        }
        cd.timer.reset();
        sfx_events.send(crate::core::events::SfxEvent {
            kind: crate::core::events::SfxKind::RangedAttack,
        });

        let dir = facing.0;
        let data_ref = data.as_deref();
        let speed_boost_mult = data_ref
            .map(|d| {
                tuning::speed_boost_mult(
                    d,
                    inventory
                        .map(|value| value.stacks(AugmentId::SpeedBoost))
                        .unwrap_or(0),
                )
            })
            .unwrap_or(1.0);
        let speed =
            BASE_RANGED_PROJECTILE_SPEED * mods.ranged_projectile_speed_mult() * speed_boost_mult;
        let greed_mult = data_ref
            .map(|d| greed_damage_mult(d, inventory, gold.0))
            .unwrap_or(1.0);
        let buff_attack = 1.0 + buff.map(|b| b.attack_bonus).unwrap_or(0.0);
        let damage = power.0 * 0.65 * mods.ranged_damage_mult() * greed_mult * buff_attack;
        let Some(d) = data_ref else {
            continue;
        };
        spawn_player_ranged_volley(
            &mut commands,
            &assets,
            d,
            player_e,
            tf.translation().truncate() + dir * 18.0,
            dir,
            speed,
            damage,
            crit.0 + greed_crit_bonus(d, inventory, gold.0),
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

pub fn spawn_player_melee_hitbox_with_mods(
    commands: &mut Commands,
    assets: &GameAssets,
    data: &GameDataRegistry,
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
    let heavy_profile = tuning::heavy_strike_profile(data, heavy_strike_stacks);
    let whirlwind_stacks = inventory
        .map(|value| value.stacks(AugmentId::Whirlwind))
        .unwrap_or(0);
    let whirlwind_damage_mult = tuning::whirlwind_damage_mult(data, whirlwind_stacks);
    let crit_enhance_stacks = inventory
        .map(|value| value.stacks(AugmentId::CritEnhance))
        .unwrap_or(0);
    let crit_profile = tuning::crit_enhance_profile(data, crit_enhance_stacks);

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
            damage: damage * heavy_profile.damage_mult * whirlwind_damage_mult,
            knockback: (360.0 + mods.melee_mastery_stacks as f32 * 12.0)
                * heavy_profile.knockback_mult,
            can_crit: true,
            crit_chance: crit_chance + crit_profile.crit_bonus,
            crit_multiplier: 1.75 + crit_profile.crit_multiplier_bonus,
        },
        ArcHitbox {
            origin: owner_pos,
            direction,
            radius: swing.reach * tuning::whirlwind_range_mult(data, whirlwind_stacks),
            half_angle_rad: if whirlwind_stacks > 0 { PI } else { half_angle },
        },
        Lifetime(Timer::from_seconds(
            MELEE_HITBOX_LIFETIME_S,
            TimerMode::Once,
        )),
        InGameEntity,
        Name::new("PlayerHitbox"),
    ));

    // Whirlwind visual effect
    if whirlwind_stacks > 0 {
        crate::gameplay::effects::particles::spawn_whirlwind_visual(commands, assets, owner_pos);
    }

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

    // SwordWave augment: spawn a ranged sword wave projectile
    let sword_wave_stacks = inventory
        .map(|value| value.stacks(AugmentId::SwordWave))
        .unwrap_or(0);
    if sword_wave_stacks > 0 && !mods.melee_sword_wave_unlocked() {
        let Some(sword_wave) = tuning::sword_wave_profile(data, sword_wave_stacks, false) else {
            return;
        };
        let sw_entity = spawn_player_sword_wave(
            commands,
            assets,
            owner,
            owner_pos + direction * (swing.reach + 12.0),
            direction,
            damage * sword_wave.damage_fraction,
        );
        if sword_wave.pierce_remaining > 0 {
            commands.entity(sw_entity).insert((
                PierceCount {
                    remaining: sword_wave.pierce_remaining,
                },
                HitTargets::default(),
            ));
        }
    }
}

pub fn spawn_player_ranged_volley(
    commands: &mut Commands,
    assets: &GameAssets,
    data: &GameDataRegistry,
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
            data,
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
    data: &GameDataRegistry,
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
    let extra_projectiles = tuning::extra_projectile_count(
        data,
        inventory
            .map(|value| value.stacks(AugmentId::ExtraProjectile))
            .unwrap_or(0),
    );
    let pierce_remaining = 0;
    let scatter_stacks = inventory
        .map(|value| value.stacks(AugmentId::Scatter))
        .unwrap_or(0);
    let crit_enhance_stacks = inventory
        .map(|value| value.stacks(AugmentId::CritEnhance))
        .unwrap_or(0);
    let crit_profile = tuning::crit_enhance_profile(data, crit_enhance_stacks);
    let final_crit_chance = crit_chance + crit_profile.crit_bonus;
    let final_crit_multiplier = 1.75 + crit_profile.crit_multiplier_bonus;

    if scatter_stacks > 0 {
        // Scatter fan visual
        crate::gameplay::effects::particles::spawn_scatter_fan(commands, assets, pos, dir);
        let Some(scatter) = tuning::scatter_profile(data, scatter_stacks) else {
            return;
        };
        let angles = scatter_angles(scatter.shots, scatter.ring);
        for angle in angles {
            let shot_dir = Mat2::from_angle(angle).mul_vec2(dir);
            queue_or_spawn_ranged_projectile(
                commands,
                assets,
                data,
                owner,
                pos,
                shot_dir,
                projectile_speed,
                damage * scatter.damage_fraction,
                final_crit_chance,
                final_crit_multiplier,
                delay_s,
                pierce_remaining,
                inventory,
            );
        }
        // ExtraProjectile must still fire alongside scatter — the previous
        // early-return swallowed the extra shots entirely.
        spawn_extra_projectiles_for_burst(
            commands,
            assets,
            data,
            owner,
            pos,
            dir,
            projectile_speed,
            damage,
            final_crit_chance,
            final_crit_multiplier,
            delay_s,
            extra_projectiles,
            pierce_remaining,
            inventory,
        );
        return;
    }

    match pattern {
        RangedVolleyPattern::Single | RangedVolleyPattern::Double => {
            queue_or_spawn_ranged_projectile(
                commands,
                assets,
                data,
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
                final_crit_chance,
                final_crit_multiplier,
                delay_s,
                pierce_remaining,
                inventory,
            );
            spawn_extra_projectiles_for_burst(
                commands,
                assets,
                data,
                owner,
                pos,
                dir,
                projectile_speed,
                damage,
                final_crit_chance,
                final_crit_multiplier,
                delay_s,
                extra_projectiles,
                pierce_remaining,
                inventory,
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
                    data,
                    owner,
                    pos,
                    shot_dir,
                    projectile_speed,
                    shot_damage,
                    final_crit_chance,
                    final_crit_multiplier,
                    delay_s,
                    pierce_remaining,
                    inventory,
                );
            }
            spawn_extra_projectiles_for_burst(
                commands,
                assets,
                data,
                owner,
                pos,
                dir,
                projectile_speed,
                damage,
                final_crit_chance,
                final_crit_multiplier,
                delay_s,
                extra_projectiles,
                pierce_remaining,
                inventory,
            );
        }
        RangedVolleyPattern::Nova => {
            queue_or_spawn_ranged_projectile(
                commands,
                assets,
                data,
                owner,
                pos,
                dir,
                projectile_speed,
                damage * 0.48,
                final_crit_chance,
                final_crit_multiplier,
                delay_s,
                pierce_remaining,
                inventory,
            );

            let base_angle = dir.y.atan2(dir.x);
            for i in 0..NOVA_PROJECTILE_COUNT {
                let angle =
                    base_angle + i as f32 / NOVA_PROJECTILE_COUNT as f32 * std::f32::consts::TAU;
                let shot_dir = Vec2::new(angle.cos(), angle.sin());
                queue_or_spawn_ranged_projectile(
                    commands,
                    assets,
                    data,
                    owner,
                    pos,
                    shot_dir,
                    projectile_speed,
                    damage * 0.20,
                    final_crit_chance,
                    final_crit_multiplier,
                    delay_s,
                    pierce_remaining,
                    inventory,
                );
            }
            spawn_extra_projectiles_for_burst(
                commands,
                assets,
                data,
                owner,
                pos,
                dir,
                projectile_speed,
                damage,
                final_crit_chance,
                final_crit_multiplier,
                delay_s,
                extra_projectiles,
                pierce_remaining,
                inventory,
            );
        }
    }
}

fn spawn_extra_projectiles_for_burst(
    commands: &mut Commands,
    assets: &GameAssets,
    data: &GameDataRegistry,
    owner: Entity,
    pos: Vec2,
    dir: Vec2,
    projectile_speed: f32,
    damage: f32,
    crit_chance: f32,
    crit_multiplier: f32,
    delay_s: f32,
    extra_projectiles: u8,
    pierce_remaining: u8,
    inventory: Option<&AugmentInventory>,
) {
    if extra_projectiles == 0 {
        return;
    }
    let stacks = inventory
        .map(|inv| inv.stacks(AugmentId::ExtraProjectile))
        .unwrap_or(0);
    let damage_fraction = tuning::extra_projectile_damage_fraction(data, stacks);
    for extra_index in 0..extra_projectiles {
        // Same direction, staggered delay so player can see each shot
        let extra_delay = delay_s + (extra_index as f32 + 1.0) * EXTRA_PROJECTILE_STAGGER_S;
        queue_or_spawn_ranged_projectile(
            commands,
            assets,
            data,
            owner,
            pos,
            dir,
            projectile_speed,
            damage * damage_fraction,
            crit_chance,
            crit_multiplier,
            extra_delay,
            pierce_remaining,
            inventory,
        );
    }
}

fn queue_or_spawn_ranged_projectile(
    commands: &mut Commands,
    assets: &GameAssets,
    data: &GameDataRegistry,
    owner: Entity,
    pos: Vec2,
    dir: Vec2,
    projectile_speed: f32,
    damage: f32,
    crit_chance: f32,
    crit_multiplier: f32,
    delay_s: f32,
    pierce_remaining: u8,
    inventory: Option<&AugmentInventory>,
) {
    let homing_stacks = inventory
        .map(|value| value.stacks(AugmentId::Homing))
        .unwrap_or(0);
    if delay_s <= 0.0 {
        spawn_ranged_projectile(
            commands,
            assets,
            data,
            owner,
            pos,
            dir,
            projectile_speed,
            damage,
            crit_chance,
            crit_multiplier,
            pierce_remaining,
            homing_stacks,
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
            crit_multiplier,
            pierce_remaining,
            homing_stacks,
        },
        InGameEntity,
        Name::new("DelayedRangedShot"),
    ));
}

fn spawn_ranged_projectile(
    commands: &mut Commands,
    assets: &GameAssets,
    data: &GameDataRegistry,
    owner: Entity,
    pos: Vec2,
    dir: Vec2,
    projectile_speed: f32,
    damage: f32,
    crit_chance: f32,
    crit_multiplier: f32,
    pierce_remaining: u8,
    homing_stacks: u8,
) {
    let projectile = projectiles::spawn_player_projectile_with_kind_and_crit(
        commands,
        assets,
        owner,
        pos,
        dir * projectile_speed,
        damage,
        crit_chance,
        crit_multiplier,
        DamageKind::PlayerRanged,
    );
    if homing_stacks > 0 {
        commands.entity(projectile).insert(
            crate::gameplay::augment::effects::HomingProjectile::from_stacks(
                data,
                homing_stacks,
                projectile_speed,
            ),
        );
    }
    let homing_pierce = crate::gameplay::augment::tuning::homing_pierce(data, homing_stacks);
    let pierce_remaining = pierce_remaining.max(homing_pierce);
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
    data: Res<GameDataRegistry>,
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
            &data,
            shot.owner,
            shot.pos,
            shot.dir,
            shot.projectile_speed,
            shot.damage,
            shot.crit_chance,
            shot.crit_multiplier,
            shot.pierce_remaining,
            shot.homing_stacks,
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
) -> Entity {
    let direction = dir.try_normalize().unwrap_or(Vec2::X);
    let size = Vec2::new(82.0, 36.0);
    let velocity = direction * SWORD_WAVE_SPEED;
    commands
        .spawn((
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
        ))
        .id()
}

fn scatter_angles(shots: usize, ring: bool) -> Vec<f32> {
    if ring {
        return (0..shots)
            .map(|index| std::f32::consts::TAU * index as f32 / shots as f32)
            .collect();
    }

    match shots {
        5 => vec![-0.36, -0.18, 0.0, 0.18, 0.36],
        3 => vec![-0.24, 0.0, 0.24],
        _ => vec![0.0],
    }
}

fn greed_damage_mult(
    data: &GameDataRegistry,
    inventory: Option<&AugmentInventory>,
    gold: u32,
) -> f32 {
    let stacks = inventory
        .map(|inv| inv.stacks(AugmentId::Greed))
        .unwrap_or(0);
    tuning::greed_damage_mult(data, stacks, gold)
}

fn greed_crit_bonus(
    data: &GameDataRegistry,
    inventory: Option<&AugmentInventory>,
    gold: u32,
) -> f32 {
    let stacks = inventory
        .map(|inv| inv.stacks(AugmentId::Greed))
        .unwrap_or(0);
    tuning::greed_crit_bonus(data, stacks, gold)
}
