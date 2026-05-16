use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentInventory, AugmentRarity};
use crate::gameplay::player::components::{
    AttackCooldown, AttackPower, CritChance, DashCooldown, Energy, Gold, Health, MoveSpeed, Player,
    RangedCooldown, RewardModifiers, SkillSlot, SkillSlots,
};
use crate::gameplay::progression::experience::PlayerLevel;
use crate::ui::tooltip::TooltipContent;
use crate::ui::widgets;

pub type CharacterSummaryItem<'a> = (
    &'a Health,
    &'a Energy,
    &'a Gold,
    &'a MoveSpeed,
    &'a AttackPower,
    &'a CritChance,
    &'a AttackCooldown,
    &'a RangedCooldown,
    &'a DashCooldown,
    Option<&'a PlayerLevel>,
    Option<&'a AugmentInventory>,
    Option<&'a SkillSlots>,
    Option<&'a RewardModifiers>,
);

#[derive(Debug, Clone)]
pub struct AugmentChip {
    pub title: String,
    pub rarity: AugmentRarity,
    pub tooltip: TooltipContent,
}

#[derive(Debug, Clone, Default)]
pub struct CharacterSummary {
    pub stats: Vec<String>,
    pub augments: Vec<AugmentChip>,
    pub augment_text_fallback: Vec<String>,
    pub skills: Vec<String>,
    pub tools: Vec<String>,
}

pub fn character_summary_from_query(
    player_q: &Query<CharacterSummaryItem<'_>, With<Player>>,
    data: Option<&GameDataRegistry>,
) -> CharacterSummary {
    player_q
        .get_single()
        .map(|item| build_character_summary(item, data))
        .unwrap_or_else(|_| CharacterSummary {
            stats: vec!["未找到角色数据".to_string()],
            augments: vec![],
            augment_text_fallback: vec!["暂无强化".to_string()],
            skills: vec!["暂无终结技".to_string()],
            tools: vec![],
        })
}

pub fn build_character_summary(
    item: CharacterSummaryItem<'_>,
    data: Option<&GameDataRegistry>,
) -> CharacterSummary {
    let (
        health,
        energy,
        gold,
        move_speed,
        attack,
        crit,
        attack_cd,
        ranged_cd,
        dash_cd,
        level,
        inventory,
        slots,
        reward_mods,
    ) = item;

    let level_line = level
        .map(|level| {
            format!(
                "等级: {}   XP: {} / {}",
                level.level, level.xp, level.xp_to_next
            )
        })
        .unwrap_or_else(|| "等级: 未记录".to_string());

    let stats = vec![
        format!("HP: {:.0} / {:.0}", health.current, health.max),
        format!("能量: {:.0} / {:.0}", energy.current, energy.max),
        format!("金币: {}", gold.0),
        level_line,
        format!("攻击: {:.0}", attack.0),
        format!("暴击: {:.0}%", crit.0 * 100.0),
        format!("移速: {:.0}", move_speed.0),
        format!("近战CD: {:.2}s", attack_cd.timer.duration().as_secs_f32()),
        format!("远程CD: {:.2}s", ranged_cd.timer.duration().as_secs_f32()),
        format!("冲刺CD: {:.2}s", dash_cd.timer.duration().as_secs_f32()),
    ];

    let (augments, augment_text_fallback) = inventory
        .map(|inventory| {
            if inventory.augments.is_empty() {
                return (vec![], vec!["暂无强化".to_string()]);
            }
            let chips: Vec<AugmentChip> = inventory
                .augments
                .iter()
                .map(|held| {
                    let (title, rarity, description) = data
                        .and_then(|registry| {
                            registry
                                .augments
                                .augments
                                .iter()
                                .find(|augment| augment.id == held.id)
                                .map(|augment| {
                                    (
                                        augment.title.clone(),
                                        augment.rarity,
                                        augment.description_for_stacks(held.stacks).to_string(),
                                    )
                                })
                        })
                        .unwrap_or_else(|| {
                            ("未知强化".to_string(), AugmentRarity::Common, "效果未配置".to_string())
                        });
                    AugmentChip {
                        title: title.clone(),
                        rarity,
                        tooltip: TooltipContent {
                            title: format!("{} Lv{}", title, held.stacks),
                            rarity: Some(rarity),
                            body: description,
                            tradeoff: None,
                            price: None,
                        },
                    }
                })
                .collect();
            (chips, vec![])
        })
        .unwrap_or_else(|| (vec![], vec!["暂无强化".to_string()]));

    let skills = slots
        .map(|slots| {
            SkillSlot::ALL
                .into_iter()
                .map(|slot| {
                    let state = slots.state(slot);
                    if !state.unlocked {
                        return format!("{}槽: 未解锁", slot.key_label());
                    }
                    match state.skill {
                        Some(skill) => {
                            let detail = data
                                .and_then(|registry| registry.skills.get(skill))
                                .map(|config| {
                                    format!(
                                        " · {:.0}能量 / {:.1}s",
                                        config.energy_cost, config.cooldown_s
                                    )
                                })
                                .unwrap_or_default();
                            format!("{}槽: {}{}", slot.key_label(), skill.label(), detail)
                        }
                        None => format!("{}槽: 空槽", slot.key_label()),
                    }
                })
                .collect()
        })
        .unwrap_or_else(|| vec!["暂无终结技".to_string()]);

    let mut tools = Vec::new();
    if let Some(mods) = reward_mods {
        if mods.talisman_charges > 0 {
            tools.push(format!("护身符 ×{}", mods.talisman_charges));
        }
    }

    CharacterSummary {
        stats,
        augments,
        augment_text_fallback,
        skills,
        tools,
    }
}

pub fn spawn_character_summary(
    parent: &mut ChildBuilder,
    assets: &GameAssets,
    summary: &CharacterSummary,
) {
    parent
        .spawn((
            widgets::summary_panel_node(),
            Name::new("CharacterSummaryPanel"),
        ))
        .with_children(|panel| {
            panel.spawn(widgets::section_heading(assets, "角色状态"));
            for line in summary.stats.iter().take(10) {
                let color = if line.starts_with("HP:") {
                    widgets::hp_color()
                } else if line.starts_with("能量:") {
                    widgets::energy_color()
                } else if line.starts_with("金币:") {
                    widgets::gold_color()
                } else {
                    Color::srgb(0.92, 0.92, 0.95)
                };
                panel.spawn(widgets::accent_text(assets, line, 15.0, color));
            }

            panel.spawn(widgets::section_heading(assets, "当前强化"));
            if summary.augments.is_empty() {
                for line in &summary.augment_text_fallback {
                    panel.spawn(widgets::muted_text(assets, line, 13.0));
                }
            } else {
                panel
                    .spawn(widgets::wrap_row_node(5.0))
                    .with_children(|row| {
                        for chip in &summary.augments {
                            row.spawn((
                                widgets::chip_node(widgets::rarity_color(chip.rarity)),
                                chip.tooltip.clone(),
                                Interaction::None,
                            ))
                            .with_children(|c| {
                                c.spawn(widgets::accent_text(
                                    assets,
                                    &chip.title,
                                    13.0,
                                    widgets::rarity_color(chip.rarity),
                                ));
                            });
                        }
                    });
            }

            panel.spawn(widgets::section_heading(assets, "终结技槽位"));
            for line in &summary.skills {
                panel.spawn(widgets::accent_text(
                    assets,
                    line,
                    14.0,
                    widgets::skill_color(),
                ));
            }

            if !summary.tools.is_empty() {
                panel.spawn(widgets::section_heading(assets, "工具"));
                for line in &summary.tools {
                    panel.spawn(widgets::accent_text(
                        assets,
                        line,
                        14.0,
                        widgets::sanctuary_color(),
                    ));
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn character_summary_contains_stats_augments_and_skills() {
        let health = Health {
            current: 70.0,
            max: 100.0,
        };
        let energy = Energy {
            current: 40.0,
            max: 80.0,
        };
        let gold = Gold(50);
        let speed = MoveSpeed(220.0);
        let attack = AttackPower(12.0);
        let crit = CritChance(0.15);
        let attack_cd = AttackCooldown {
            timer: Timer::new(Duration::from_millis(500), TimerMode::Once),
            base_duration_s: 0.5,
        };
        let ranged_cd = RangedCooldown {
            timer: Timer::new(Duration::from_millis(700), TimerMode::Once),
            base_duration_s: 0.7,
        };
        let dash_cd = DashCooldown {
            timer: Timer::new(Duration::from_millis(900), TimerMode::Once),
            base_duration_s: 0.9,
        };
        let level = PlayerLevel::default();
        let mut inventory = AugmentInventory::default();
        inventory.add(crate::gameplay::augment::data::AugmentId::Piercing);
        let skills = SkillSlots::default();
        let mods = RewardModifiers {
            talisman_charges: 2,
            ..Default::default()
        };

        let summary = build_character_summary(
            (
                &health,
                &energy,
                &gold,
                &speed,
                &attack,
                &crit,
                &attack_cd,
                &ranged_cd,
                &dash_cd,
                Some(&level),
                Some(&inventory),
                Some(&skills),
                Some(&mods),
            ),
            None,
        );

        assert!(summary.stats.iter().any(|line| line.contains("HP")));
        assert_eq!(summary.augments.len(), 1);
        assert_eq!(summary.augments[0].title, "未知强化");
        assert!(summary.skills.iter().any(|line| line.contains("1槽")));
        assert!(summary.tools.iter().any(|line| line.contains("护身符")));
    }
}
