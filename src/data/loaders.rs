use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use bevy::prelude::*;

use crate::data::definitions::*;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::enemy::components::EnemyType;

pub fn load_all_configs(mut commands: Commands) {
    // Per-file fallback: a single malformed RON only reverts that one config to
    // its built-in default, not the entire registry. The failing file is named
    // in the warning so it can be located.
    let defaults = default_registry();
    let registry = GameDataRegistry {
        player: load_or_default("assets/configs/player.ron", defaults.player),
        enemies: load_or_default("assets/configs/enemies.ron", defaults.enemies),
        bosses: load_or_default("assets/configs/boss.ron", defaults.bosses),
        rewards: load_or_default("assets/configs/rewards.ron", defaults.rewards),
        rooms: load_or_default("assets/configs/rooms.ron", defaults.rooms),
        balance: load_or_default("assets/configs/game_balance.ron", defaults.balance),
        augments: load_or_default("assets/configs/augments.ron", defaults.augments),
        skills: load_or_default("assets/configs/skills.ron", defaults.skills),
        events: load_or_default("assets/configs/events.ron", defaults.events),
        shop: load_or_default("assets/configs/shop.ron", defaults.shop),
        economy: load_or_default("assets/configs/balance.ron", defaults.economy),
        elite_affixes: load_or_default("assets/configs/elite_affixes.ron", defaults.elite_affixes),
        audio: load_or_default("assets/configs/audio.ron", defaults.audio),
        effects: load_or_default("assets/configs/effects.ron", defaults.effects),
    };
    commands.insert_resource(registry);
}

fn load_or_default<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>, fallback: T) -> T {
    let path = path.as_ref();
    match load_ron::<T>(path) {
        Ok(value) => value,
        Err(err) => {
            warn!(
                "config load failed for {}; using built-in default for this file only: {err:?}",
                path.display()
            );
            fallback
        }
    }
}

pub fn load_ron<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let value =
        ron::from_str::<T>(&content).with_context(|| format!("parse ron {}", path.display()))?;
    Ok(value)
}

pub(crate) fn default_registry() -> GameDataRegistry {
    GameDataRegistry {
        player: PlayerConfig {
            max_hp: 100.0,
            move_speed: 260.0,
            attack_power: 18.0,
            attack_cooldown_s: 0.70,
            ranged_cooldown_s: 0.80,
            dash_cooldown_s: 1.2,
            dash_speed: 680.0,
            dash_duration_s: 0.12,
            invincibility_s: 0.35,
            crit_chance: 0.05,
            energy_max: 100.0,
            melee_charge_gain: 8.0,
            ranged_charge_gain: 4.0,
            kill_charge_gain: 10.0,
            elite_kill_charge_gain: 22.0,
            perfect_dash_charge_gain: 15.0,
            combo_charge_gain: 10.0,
            finisher_charge_cost: 100.0,
            skill1_cooldown_s: 1.1,
            ranged_base_cooldown_s: 0.80,
            ranged_min_cooldown_s: 0.80,
            ranged_ramp_max: 1,
        },
        enemies: EnemiesConfig {
            melee_chaser: EnemyStatsConfig {
                max_hp: 44.0,
                move_speed: 172.0,
                attack_damage: 14.0,
                attack_cooldown_s: 0.82,
                aggro_range: 480.0,
                attack_range: 42.0,
                projectile_speed: 0.0,
            },
            lobber: EnemyStatsConfig {
                max_hp: 36.0,
                move_speed: 112.0,
                attack_damage: 12.0,
                attack_cooldown_s: 1.25,
                aggro_range: 620.0,
                attack_range: 440.0,
                projectile_speed: 390.0,
            },
            ranged_shooter: EnemyStatsConfig {
                max_hp: 34.0,
                move_speed: 130.0,
                attack_damage: 11.0,
                attack_cooldown_s: 1.00,
                aggro_range: 580.0,
                attack_range: 380.0,
                projectile_speed: 470.0,
            },
            charger: EnemyStatsConfig {
                max_hp: 62.0,
                move_speed: 120.0,
                attack_damage: 21.0,
                attack_cooldown_s: 0.80,
                aggro_range: 560.0,
                attack_range: 350.0,
                projectile_speed: 0.0,
            },
            charger_config: ChargerConfig {
                charge_duration_s: 0.5,
                charge_speed_mult: 5.4,
                wall_stun_s: 1.5,
                cooldown_s: 0.5,
                contact_damage_mult: 1.00,
                contact_knockback: 220.0,
            },
            flanker: EnemyStatsConfig {
                max_hp: 46.0,
                move_speed: 195.0,
                attack_damage: 15.0,
                attack_cooldown_s: 0.82,
                aggro_range: 700.0,
                attack_range: 56.0,
                projectile_speed: 0.0,
            },
            sniper: EnemyStatsConfig {
                max_hp: 44.0,
                move_speed: 108.0,
                attack_damage: 17.0,
                attack_cooldown_s: 1.45,
                aggro_range: 860.0,
                attack_range: 620.0,
                projectile_speed: 840.0,
            },
            support_caster: EnemyStatsConfig {
                max_hp: 38.0,
                move_speed: 108.0,
                attack_damage: 9.0,
                attack_cooldown_s: 2.20,
                aggro_range: 700.0,
                attack_range: 240.0,
                projectile_speed: 340.0,
            },
            bomber: EnemyStatsConfig {
                max_hp: 30.0,
                move_speed: 240.0,
                attack_damage: 28.0,
                attack_cooldown_s: 1.50,
                aggro_range: 560.0,
                attack_range: 55.0,
                projectile_speed: 0.0,
            },
            shielder: EnemyStatsConfig {
                max_hp: 72.0,
                move_speed: 80.0,
                attack_damage: 16.0,
                attack_cooldown_s: 1.20,
                aggro_range: 540.0,
                attack_range: 40.0,
                projectile_speed: 0.0,
            },
            summoner: EnemyStatsConfig {
                max_hp: 28.0,
                move_speed: 95.0,
                attack_damage: 8.0,
                attack_cooldown_s: 4.0,
                aggro_range: 760.0,
                attack_range: 500.0,
                projectile_speed: 320.0,
            },
        },
        bosses: BossesConfig {
            floor_1: BossFloorConfig {
                max_hp: 330.0,
                move_speed: 95.0,
                contact_damage: 14.0,
                phase_thresholds: vec![0.60, 0.30],
                projectile_speed: 430.0,
            },
            floor_2: BossFloorConfig {
                max_hp: 450.0,
                move_speed: 130.0,
                contact_damage: 15.0,
                phase_thresholds: vec![0.68, 0.34],
                projectile_speed: 470.0,
            },
            floor_3: BossFloorConfig {
                max_hp: 600.0,
                move_speed: 175.0,
                contact_damage: 18.0,
                phase_thresholds: vec![0.70, 0.35],
                projectile_speed: 505.0,
            },
            floor_4: BossFloorConfig {
                max_hp: 620.0,
                move_speed: 82.0,
                contact_damage: 16.0,
                phase_thresholds: vec![0.72, 0.38],
                projectile_speed: 540.0,
            },
        },
        rewards: RewardsConfig {
            rewards: vec![],
            scaling: RewardScalingConfig::default_config(),
        },
        rooms: RoomGenConfig {
            room_sequence: vec![],
        },
        balance: GameBalanceConfig {
            difficulty_per_floor: 0.16,
            enemy_count_normal_room: 4,
            reward_rooms_give_choice: true,
            boss_room_gives_victory: false,
            total_floors: 4,
            floor_rooms: 10,
            enemy_pools_by_floor: vec![],
            elite_chance: 0.18,
            elite_hp_mult: 2.0,
            elite_damage_mult: 1.55,
            elite_gold_bonus: 12,
            use_sprite_textures: false,
            enemy_count_by_floor: vec![4, 5, 5, 6],
            elite_chance_by_floor: vec![0.0, 0.0, 0.18, 0.32],
            floor_growth_curves: vec![
                FloorGrowthCurve {
                    hp: 0.0,
                    damage: 0.0,
                    cooldown: 0.0,
                    projectile: 0.0,
                },
                FloorGrowthCurve {
                    hp: 0.625,
                    damage: 0.50,
                    cooldown: 0.1875,
                    projectile: 0.3125,
                },
                FloorGrowthCurve {
                    hp: 3.4375,
                    damage: 1.25,
                    cooldown: 0.625,
                    projectile: 0.75,
                },
                FloorGrowthCurve {
                    hp: 6.5625,
                    damage: 2.375,
                    cooldown: 1.125,
                    projectile: 1.25,
                },
            ],
            enemy_type_curves: vec![
                EnemyTypeCurve {
                    enemy: EnemyType::MeleeChaser,
                    hp: 1.08,
                    damage: 1.0,
                    cooldown: 1.0,
                    projectile: 1.0,
                    aggro_bonus: 0.0,
                },
                EnemyTypeCurve {
                    enemy: EnemyType::Lobber,
                    hp: 1.10,
                    damage: 1.0,
                    cooldown: 0.98,
                    projectile: 1.08,
                    aggro_bonus: 0.0,
                },
                EnemyTypeCurve {
                    enemy: EnemyType::RangedShooter,
                    hp: 1.15,
                    damage: 1.0,
                    cooldown: 0.96,
                    projectile: 1.05,
                    aggro_bonus: 0.0,
                },
                EnemyTypeCurve {
                    enemy: EnemyType::Charger,
                    hp: 1.20,
                    damage: 1.08,
                    cooldown: 1.0,
                    projectile: 1.0,
                    aggro_bonus: 80.0,
                },
                EnemyTypeCurve {
                    enemy: EnemyType::Flanker,
                    hp: 1.12,
                    damage: 1.10,
                    cooldown: 1.0,
                    projectile: 1.0,
                    aggro_bonus: 0.0,
                },
                EnemyTypeCurve {
                    enemy: EnemyType::Sniper,
                    hp: 1.18,
                    damage: 1.12,
                    cooldown: 1.0,
                    projectile: 1.0,
                    aggro_bonus: 0.0,
                },
            ],
            floor1_easing: Floor1Easing::default(),
        },
        augments: AugmentsConfig { augments: vec![] },
        skills: SkillsConfig { skills: vec![] },
        events: EventsConfig { events: vec![] },
        shop: ShopConfig::default(),
        economy: EconomyConfig::default(),
        elite_affixes: EliteAffixesConfig { affixes: vec![] },
        audio: AudioConfig::default(),
        effects: EffectsConfig::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use crate::gameplay::augment::data::{AugmentId, AugmentRarity};
    use crate::gameplay::enemy::components::{EliteAffix, EnemyType};
    use crate::gameplay::player::components::SkillType;

    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    /// Guard rail: `default_registry()` is the fallback used when an asset RON
    /// fails to load. It MUST stay in sync with the shipped `.ron` files so a
    /// parse error doesn't silently downgrade balance to a stale snapshot.
    #[test]
    fn loaders_fallback_matches_shipped_ron_files() {
        let fallback = default_registry();

        let player: PlayerConfig = load_ron("assets/configs/player.ron").unwrap();
        close(player.max_hp, fallback.player.max_hp);
        close(player.move_speed, fallback.player.move_speed);
        close(player.attack_power, fallback.player.attack_power);
        close(player.attack_cooldown_s, fallback.player.attack_cooldown_s);
        close(player.ranged_cooldown_s, fallback.player.ranged_cooldown_s);
        close(player.dash_cooldown_s, fallback.player.dash_cooldown_s);
        close(player.dash_speed, fallback.player.dash_speed);
        close(player.dash_duration_s, fallback.player.dash_duration_s);
        close(player.invincibility_s, fallback.player.invincibility_s);
        close(player.crit_chance, fallback.player.crit_chance);
        close(player.energy_max, fallback.player.energy_max);
        close(player.melee_charge_gain, fallback.player.melee_charge_gain);
        close(
            player.ranged_charge_gain,
            fallback.player.ranged_charge_gain,
        );
        close(player.kill_charge_gain, fallback.player.kill_charge_gain);
        close(
            player.elite_kill_charge_gain,
            fallback.player.elite_kill_charge_gain,
        );
        close(
            player.perfect_dash_charge_gain,
            fallback.player.perfect_dash_charge_gain,
        );
        close(player.combo_charge_gain, fallback.player.combo_charge_gain);
        close(
            player.finisher_charge_cost,
            fallback.player.finisher_charge_cost,
        );

        let enemies: EnemiesConfig = load_ron("assets/configs/enemies.ron").unwrap();
        for (name, ron, fb) in [
            (
                "melee_chaser",
                &enemies.melee_chaser,
                &fallback.enemies.melee_chaser,
            ),
            ("lobber", &enemies.lobber, &fallback.enemies.lobber),
            (
                "ranged_shooter",
                &enemies.ranged_shooter,
                &fallback.enemies.ranged_shooter,
            ),
            ("charger", &enemies.charger, &fallback.enemies.charger),
            ("flanker", &enemies.flanker, &fallback.enemies.flanker),
            ("sniper", &enemies.sniper, &fallback.enemies.sniper),
            (
                "support_caster",
                &enemies.support_caster,
                &fallback.enemies.support_caster,
            ),
            ("bomber", &enemies.bomber, &fallback.enemies.bomber),
            ("shielder", &enemies.shielder, &fallback.enemies.shielder),
            ("summoner", &enemies.summoner, &fallback.enemies.summoner),
        ] {
            assert!(
                (ron.max_hp - fb.max_hp).abs() < 0.001,
                "{name}.max_hp mismatch"
            );
            assert!(
                (ron.move_speed - fb.move_speed).abs() < 0.001,
                "{name}.move_speed mismatch"
            );
            assert!(
                (ron.attack_damage - fb.attack_damage).abs() < 0.001,
                "{name}.attack_damage mismatch"
            );
            assert!(
                (ron.attack_cooldown_s - fb.attack_cooldown_s).abs() < 0.001,
                "{name}.attack_cooldown_s mismatch"
            );
            assert!(
                (ron.aggro_range - fb.aggro_range).abs() < 0.001,
                "{name}.aggro_range mismatch"
            );
            assert!(
                (ron.attack_range - fb.attack_range).abs() < 0.001,
                "{name}.attack_range mismatch"
            );
            assert!(
                (ron.projectile_speed - fb.projectile_speed).abs() < 0.001,
                "{name}.projectile_speed mismatch"
            );
        }
        close(
            enemies.charger_config.charge_duration_s,
            fallback.enemies.charger_config.charge_duration_s,
        );
        close(
            enemies.charger_config.charge_speed_mult,
            fallback.enemies.charger_config.charge_speed_mult,
        );
        close(
            enemies.charger_config.wall_stun_s,
            fallback.enemies.charger_config.wall_stun_s,
        );
        close(
            enemies.charger_config.cooldown_s,
            fallback.enemies.charger_config.cooldown_s,
        );
        close(
            enemies.charger_config.contact_damage_mult,
            fallback.enemies.charger_config.contact_damage_mult,
        );
        close(
            enemies.charger_config.contact_knockback,
            fallback.enemies.charger_config.contact_knockback,
        );

        let bosses: BossesConfig = load_ron("assets/configs/boss.ron").unwrap();
        for (name, ron, fb) in [
            ("floor_1", &bosses.floor_1, &fallback.bosses.floor_1),
            ("floor_2", &bosses.floor_2, &fallback.bosses.floor_2),
            ("floor_3", &bosses.floor_3, &fallback.bosses.floor_3),
            ("floor_4", &bosses.floor_4, &fallback.bosses.floor_4),
        ] {
            assert!(
                (ron.max_hp - fb.max_hp).abs() < 0.001,
                "boss {name}.max_hp mismatch (ron {} vs fb {})",
                ron.max_hp,
                fb.max_hp
            );
            assert!(
                (ron.move_speed - fb.move_speed).abs() < 0.001,
                "boss {name}.move_speed mismatch"
            );
            assert!(
                (ron.contact_damage - fb.contact_damage).abs() < 0.001,
                "boss {name}.contact_damage mismatch"
            );
            assert!(
                (ron.projectile_speed - fb.projectile_speed).abs() < 0.001,
                "boss {name}.projectile_speed mismatch"
            );
            assert_eq!(
                ron.phase_thresholds, fb.phase_thresholds,
                "boss {name}.phase_thresholds mismatch"
            );
        }

        let balance: GameBalanceConfig = load_ron("assets/configs/game_balance.ron").unwrap();
        close(
            balance.difficulty_per_floor,
            fallback.balance.difficulty_per_floor,
        );
        assert_eq!(
            balance.enemy_count_normal_room,
            fallback.balance.enemy_count_normal_room
        );
        assert_eq!(balance.total_floors, fallback.balance.total_floors);
        assert_eq!(balance.floor_rooms, fallback.balance.floor_rooms);
        close(balance.elite_chance, fallback.balance.elite_chance);
        close(balance.elite_hp_mult, fallback.balance.elite_hp_mult);
        close(
            balance.elite_damage_mult,
            fallback.balance.elite_damage_mult,
        );
        assert_eq!(
            balance.elite_gold_bonus, fallback.balance.elite_gold_bonus,
            "elite_gold_bonus mismatch"
        );
        assert_eq!(
            balance.use_sprite_textures, fallback.balance.use_sprite_textures,
            "use_sprite_textures mismatch"
        );

        let economy: EconomyConfig = load_ron("assets/configs/balance.ron").unwrap();
        assert_eq!(
            economy.normal_gold, fallback.economy.normal_gold,
            "normal_gold mismatch"
        );
        assert_eq!(economy.elite_gold, fallback.economy.elite_gold);
        assert_eq!(economy.boss_gold, fallback.economy.boss_gold);
        assert_eq!(economy.floor_income, fallback.economy.floor_income);
        assert_eq!(
            economy.xp_curve, fallback.economy.xp_curve,
            "xp_curve mismatch (ron {:?} vs fb {:?})",
            economy.xp_curve, fallback.economy.xp_curve
        );

        let effects: EffectsConfig = load_ron("assets/configs/effects.ron").unwrap();
        assert_eq!(
            effects.hit_particle_count,
            fallback.effects.hit_particle_count
        );
        assert_eq!(
            effects.death_particle_count, fallback.effects.death_particle_count,
            "death_particle_count mismatch"
        );

        let audio: AudioConfig = load_ron("assets/configs/audio.ron").unwrap();
        close(audio.master_volume, fallback.audio.master_volume);
        close(audio.sfx_volume, fallback.audio.sfx_volume);
        close(audio.bgm_volume, fallback.audio.bgm_volume);
        close(audio.pitch_variation, fallback.audio.pitch_variation);
    }

    // Regression: floor 1 must NOT spawn floor 2/3/4 enemies. The old
    // `choose_enemy_types` returned a flat `enemy_types` list verbatim for
    // every floor, so floor 1 spawned the full roster. Pools are now per-floor
    // cumulative; floor 1 = {MeleeChaser, Lobber, Charger} only.
    #[test]
    fn floor1_enemy_pool_excludes_higher_floor_enemies() {
        use crate::gameplay::enemy::spawner::choose_enemy_types;

        // Empty config -> built-in spec §7.1 table.
        let mut reg = default_registry();
        reg.balance.enemy_pools_by_floor = vec![];
        assert_floor_gating(&reg);

        // RON-provided config must gate identically.
        reg.balance = load_ron("assets/configs/game_balance.ron").unwrap();
        assert_floor_gating(&reg);

        fn assert_floor_gating(reg: &GameDataRegistry) {
            let f1 = choose_enemy_types(reg, 1);
            assert!(f1.contains(&EnemyType::MeleeChaser));
            assert!(f1.contains(&EnemyType::Lobber));
            assert!(f1.contains(&EnemyType::Charger));
            assert!(
                !f1.iter().any(|e| matches!(
                    e,
                    EnemyType::RangedShooter
                        | EnemyType::Flanker
                        | EnemyType::Bomber
                        | EnemyType::Sniper
                        | EnemyType::Shielder
                        | EnemyType::SupportCaster
                        | EnemyType::Summoner
                )),
                "floor 1 must not include floor 2/3/4 enemies, got {f1:?}"
            );

            let f2 = choose_enemy_types(reg, 2);
            assert!(f2.contains(&EnemyType::RangedShooter));
            assert!(f2.contains(&EnemyType::Flanker));
            assert!(!f2.contains(&EnemyType::Sniper));

            let f4 = choose_enemy_types(reg, 4);
            for e in [
                EnemyType::MeleeChaser,
                EnemyType::Lobber,
                EnemyType::Charger,
                EnemyType::RangedShooter,
                EnemyType::Flanker,
                EnemyType::Bomber,
                EnemyType::Sniper,
                EnemyType::Shielder,
                EnemyType::SupportCaster,
                EnemyType::Summoner,
            ] {
                assert!(f4.contains(&e), "floor 4 must include {e:?}");
            }
        }
    }

    #[test]
    fn phase3_augments_config_is_complete_three_level_matrix() {
        let config: AugmentsConfig = load_ron("assets/configs/augments.ron").unwrap();
        assert_eq!(config.augments.len(), 30);

        let mut ids = BTreeSet::new();
        for augment in &config.augments {
            assert!(ids.insert(format!("{:?}", augment.id)));
            assert_eq!(augment.levels.len(), 3, "{:?}", augment.id);
            assert_eq!(augment.max_stacks(), 3, "{:?}", augment.id);
            assert!(!augment.description_for_stacks(1).is_empty());
            assert!(!augment.description_for_stacks(2).is_empty());
            assert!(!augment.description_for_stacks(3).is_empty());
            let expected_cost = match augment.rarity {
                AugmentRarity::Common => 80,
                AugmentRarity::Elite => 150,
                AugmentRarity::Legendary => 250,
            };
            assert_eq!(augment.shop_cost, expected_cost, "{:?}", augment.id);
        }

        let by_id = |id| {
            config
                .augments
                .iter()
                .find(|augment| augment.id == id)
                .unwrap_or_else(|| panic!("missing {id:?}"))
        };
        close(
            *by_id(AugmentId::HeavyStrike).levels[2]
                .params
                .get("damage")
                .unwrap(),
            0.30,
        );
        close(
            *by_id(AugmentId::Freeze).levels[2]
                .params
                .get("shatter")
                .unwrap(),
            0.50,
        );
        close(
            *by_id(AugmentId::DashShield).levels[2]
                .params
                .get("charges")
                .unwrap(),
            3.0,
        );
        close(
            *by_id(AugmentId::Phoenix).levels[2]
                .params
                .get("revive")
                .unwrap(),
            1.0,
        );
    }

    #[test]
    fn phase3_skills_config_matches_nine_finishers_and_energy_tiers() {
        let config: SkillsConfig = load_ron("assets/configs/skills.ron").unwrap();
        assert_eq!(config.skills.len(), 9);

        for skill in [
            SkillType::GroundSlam,
            SkillType::BladeDance,
            SkillType::ExecutionBlade,
            SkillType::BulletBarrage,
            SkillType::FrostField,
            SkillType::MeteorFall,
            SkillType::WarCry,
            SkillType::LifeDrain,
            SkillType::TimeRift,
        ] {
            assert!(config.get(skill).is_some(), "missing {skill:?}");
        }

        let light = config.get(SkillType::WarCry).unwrap();
        assert_eq!(light.tier, SkillTier::Light);
        close(light.energy_cost, 60.0);
        close(light.cooldown_s, 8.0);

        let medium = config.get(SkillType::LifeDrain).unwrap();
        assert_eq!(medium.tier, SkillTier::Medium);
        close(medium.energy_cost, 80.0);
        close(medium.cooldown_s, 15.0);

        let heavy = config.get(SkillType::MeteorFall).unwrap();
        assert_eq!(heavy.tier, SkillTier::Heavy);
        assert!(heavy.consumes_all_energy);
        close(heavy.min_energy, 80.0);
        close(heavy.cooldown_s, 25.0);
    }

    #[test]
    fn phase3_events_shop_economy_and_enemy_configs_match_spec_counts() {
        let events: EventsConfig = load_ron("assets/configs/events.ron").unwrap();
        assert_eq!(events.events.len(), 17);
        assert_eq!(
            events
                .events
                .iter()
                .filter(|event| event.category == EventCategory::Puzzle)
                .count(),
            3
        );
        assert_eq!(
            events
                .events
                .iter()
                .filter(|event| event.category == EventCategory::NonCombat)
                .count(),
            10
        );
        assert_eq!(
            events
                .events
                .iter()
                .filter(|event| event.category == EventCategory::Combat)
                .count(),
            4
        );
        for (id, gold, xp) in [
            ("bullet_maze", 30, 0),
            ("memory_blocks", 0, 35),
            ("timed_collect", 25, 25),
        ] {
            let puzzle = events
                .events
                .iter()
                .find(|event| event.id == id)
                .and_then(|event| event.puzzle.as_ref())
                .unwrap_or_else(|| panic!("missing puzzle config for {id}"));
            assert!(puzzle.target_count > 0);
            assert!(puzzle.time_limit_s > 0.0);
            assert_eq!(puzzle.gold_reward, gold);
            assert_eq!(puzzle.xp_reward, xp);
        }

        let shop: ShopConfig = load_ron("assets/configs/shop.ron").unwrap();
        assert_eq!(shop.heal_price, 40);
        assert_eq!(shop.energy_price, 30);
        assert_eq!(shop.max_hp_price, 80);
        assert_eq!(shop.attack_power_price, 80);
        assert_eq!(shop.common_augment_price, 80);
        assert_eq!(shop.elite_augment_price, 150);
        assert_eq!(shop.legendary_augment_price, 250);
        assert_eq!(shop.augment_upgrade_price, 120);
        assert_eq!(shop.skill_price, 180);
        assert_eq!(shop.refresh_first_cost, 0);
        assert_eq!(shop.refresh_base_cost, 30);
        assert_eq!(shop.refresh_increment, 15);

        let economy: EconomyConfig = load_ron("assets/configs/balance.ron").unwrap();
        assert_eq!(economy.normal_gold, [3, 6]);
        assert_eq!(economy.elite_gold, [12, 20]);
        assert_eq!(economy.boss_gold, [30, 50]);
        assert_eq!(economy.floor_income, [100, 180]);
        assert!(!economy.xp_curve.is_empty());

        let enemies: EnemiesConfig = load_ron("assets/configs/enemies.ron").unwrap();
        assert!(enemies.lobber.projectile_speed > 0.0);
        assert!(enemies.charger.attack_cooldown_s <= 0.80);
        assert!(enemies.bomber.move_speed >= 240.0);
        assert!(enemies.shielder.attack_damage >= 16.0);

        let balance: GameBalanceConfig = load_ron("assets/configs/game_balance.ron").unwrap();
        assert_eq!(balance.floor_rooms, 10);
        assert_eq!(balance.enemy_pools_by_floor.len(), 4);
        assert_eq!(
            balance.enemy_pools_by_floor[0],
            vec![
                EnemyType::MeleeChaser,
                EnemyType::Lobber,
                EnemyType::Charger
            ]
        );

        let affixes: EliteAffixesConfig = load_ron("assets/configs/elite_affixes.ron").unwrap();
        assert_eq!(affixes.affixes.len(), 6);
        for affix in [
            EliteAffix::Swift,
            EliteAffix::Splitting,
            EliteAffix::Shielded,
            EliteAffix::Vampiric,
            EliteAffix::Berserk,
            EliteAffix::Teleporting,
        ] {
            assert!(
                affixes.affixes.iter().any(|config| config.affix == affix),
                "missing {affix:?}"
            );
        }
    }
}
