use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use lightyear::prelude::Replicated;

use crate::coop::components::{CoopSessionState, LocalControlled};
use crate::coop::net::{CoopNetConfig, NetMode};
use crate::core::assets::GameAssets;
use crate::data::registry::GameDataRegistry;
use crate::gameplay::enemy::components::Enemy;
use crate::gameplay::enemy::components::{EnemyKind, EnemyType};
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomId, RoomType};
use crate::gameplay::player::components::{
    DashCooldown, Energy, Gold, Health, Player, PlayerSkillState, SkillSlot, SkillSlots,
};
use crate::gameplay::progression::experience::PlayerLevel;
use crate::gameplay::progression::floor::FloorNumber;
use crate::states::RoomState;
use crate::ui::widgets;
use crate::utils::entity::safe_despawn_recursive;

#[derive(Component)]
pub struct HudUi;

#[derive(Component)]
pub struct HealthFill;

#[derive(Component)]
pub struct HealthText;

#[derive(Component)]
pub struct GoldText;

#[derive(Component)]
pub struct EnergyText;

#[derive(Component)]
pub struct EnergyFill;

#[derive(Component)]
pub struct DashText;

#[derive(Component)]
pub struct DashIconFill;

#[derive(Component)]
pub struct SkillOverlay;

#[derive(Component, Debug, Clone, Copy)]
pub struct SkillSlotPanel(pub SkillSlot);

#[derive(Component, Debug, Clone, Copy)]
pub struct SkillSlotName(pub SkillSlot);

#[derive(Component, Debug, Clone, Copy)]
pub struct SkillSlotKey(pub SkillSlot);

#[derive(Component)]
pub struct FloorText;

#[derive(Component)]
pub struct ExperienceFill;

#[derive(Component)]
pub struct ExperienceText;

#[derive(Component)]
pub struct LevelText;

#[derive(Component)]
pub struct BarAnimState {
    pub current: f32,
    pub target: f32,
}

#[derive(Component)]
pub struct BossHealthBar;

#[derive(Component)]
pub struct BossHealthFill;

#[derive(Component)]
pub struct RoomText;

#[derive(Component)]
pub struct EnemyCountText;

#[derive(Component)]
pub struct HintText;

#[derive(Component)]
pub struct MinimapRoot;

#[derive(Component)]
pub struct StageProgressText;

pub fn setup_hud(mut commands: Commands, assets: Res<GameAssets>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ..default()
            },
            HudUi,
            Name::new("HudRoot"),
        ))
        .with_children(|root| {
            root.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.0)),
                    ..default()
                },
                SkillOverlay,
                Name::new("SkillOverlay"),
            ));

            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0),
                    top: Val::Px(12.0),
                    row_gap: Val::Px(6.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ..default()
            })
            .with_children(|col| {
                col.spawn(widgets::title_text(&assets, "生命", 14.0));
                col.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(210.0),
                        height: Val::Px(16.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.15, 0.15, 0.18)),
                    ..default()
                })
                .with_children(|bar| {
                    bar.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgb(0.20, 0.85, 0.30)),
                            ..default()
                        },
                        HealthFill,
                        BarAnimState {
                            current: 1.0,
                            target: 1.0,
                        },
                    ));
                });
                col.spawn((
                    widgets::title_text(&assets, "HP: 100 / 100", 15.0),
                    HealthText,
                ));
                // Experience bar
                col.spawn((widgets::title_text(&assets, "Lv.1", 14.0), LevelText));
                col.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(180.0),
                        height: Val::Px(8.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.12, 0.12, 0.18)),
                    ..default()
                })
                .with_children(|bar| {
                    bar.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgb(0.55, 0.35, 0.95)),
                            ..default()
                        },
                        ExperienceFill,
                        BarAnimState {
                            current: 0.0,
                            target: 0.0,
                        },
                    ));
                });
                col.spawn((widgets::body_text(&assets, "0 / 40", 12.0), ExperienceText));
                col.spawn(NodeBundle {
                    style: Style {
                        margin: UiRect::top(Val::Px(6.0)),
                        padding: UiRect::all(Val::Px(8.0)),
                        row_gap: Val::Px(4.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.22)),
                    ..default()
                })
                .with_children(|panel| {
                    panel.spawn(widgets::title_text(&assets, "属性", 13.0));
                    panel.spawn((widgets::title_text(&assets, "金币: 0", 12.0), GoldText));
                    panel.spawn((
                        widgets::title_text(&assets, "能量: 100 / 100（暂未启用）", 12.0),
                        EnergyText,
                    ));
                    panel
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(190.0),
                                height: Val::Px(12.0),
                                margin: UiRect::top(Val::Px(2.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgb(0.10, 0.13, 0.18)),
                            ..default()
                        })
                        .with_children(|bar| {
                            bar.spawn((
                                NodeBundle {
                                    style: Style {
                                        width: Val::Percent(0.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    background_color: BackgroundColor(Color::srgb(
                                        0.24, 0.72, 0.96,
                                    )),
                                    ..default()
                                },
                                EnergyFill,
                                BarAnimState {
                                    current: 0.0,
                                    target: 0.0,
                                },
                            ));
                        });
                });

                col.spawn(NodeBundle {
                    style: Style {
                        margin: UiRect::top(Val::Px(4.0)),
                        column_gap: Val::Px(12.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(40.0),
                            height: Val::Px(40.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgb(0.10, 0.12, 0.16)),
                        ..default()
                    })
                    .with_children(|icon| {
                        icon.spawn((
                            NodeBundle {
                                style: Style {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    bottom: Val::Px(0.0),
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgb(0.16, 0.74, 0.95)),
                                ..default()
                            },
                            DashIconFill,
                        ));
                        icon.spawn(widgets::title_text(&assets, ">>", 16.0));
                    });

                    row.spawn((widgets::title_text(&assets, "冲刺：就绪", 14.0), DashText));
                });

                col.spawn(NodeBundle {
                    style: Style {
                        margin: UiRect::top(Val::Px(4.0)),
                        column_gap: Val::Px(6.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    for slot in SkillSlot::ALL {
                        row.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(62.0),
                                    height: Val::Px(52.0),
                                    padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgb(0.14, 0.16, 0.20)),
                                ..default()
                            },
                            SkillSlotPanel(slot),
                        ))
                        .with_children(|slot_ui| {
                            slot_ui.spawn((
                                widgets::title_text(&assets, "LOCK", 11.0),
                                SkillSlotName(slot),
                            ));
                            slot_ui.spawn((
                                widgets::title_text(&assets, slot.key_label(), 11.0),
                                SkillSlotKey(slot),
                            ));
                        });
                    }
                });

                col.spawn((
                    widgets::title_text(&assets, "楼层：第 1 层", 14.0),
                    FloorText,
                ));
                col.spawn((widgets::title_text(&assets, "房间：起始", 14.0), RoomText));
                col.spawn((
                    widgets::title_text(&assets, "敌人：0", 14.0),
                    EnemyCountText,
                ));
                col.spawn((
                    widgets::title_text(
                        &assets,
                        "提示：长按鼠标左右键持续攻击，靠近门后按 E 交互",
                        14.0,
                    ),
                    HintText,
                ));
            });

            root.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        top: Val::Px(10.0),
                        left: Val::Percent(25.0),
                        width: Val::Percent(50.0),
                        height: Val::Px(16.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                    visibility: Visibility::Hidden,
                    ..default()
                },
                BossHealthBar,
            ))
            .with_children(|bar| {
                bar.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgb(0.85, 0.20, 0.90)),
                        ..default()
                    },
                    BossHealthFill,
                ));
            });

            root.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        right: Val::Px(16.0),
                        top: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(6.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.28)),
                    ..default()
                },
                MinimapRoot,
                Name::new("MinimapRoot"),
            ))
            .with_children(|mm| {
                mm.spawn(widgets::title_text(&assets, "关卡进度", 16.0));
                mm.spawn((
                    widgets::title_text(&assets, "第1层：1/1", 15.0),
                    StageProgressText,
                ));
            });
        });
}

pub fn update_health_bar(
    player_q: Query<&Health, (With<Player>, With<LocalControlled>)>,
    time: Res<Time>,
    registry: Option<Res<GameDataRegistry>>,
    mut fill_q: Query<(&mut Style, &mut BarAnimState), With<HealthFill>>,
) {
    let Ok(hp) = player_q.get_single() else {
        return;
    };
    let Ok((mut style, mut anim)) = fill_q.get_single_mut() else {
        return;
    };
    let ratio = if hp.max > 0.0 {
        (hp.current / hp.max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    anim.target = ratio;
    let speed = registry
        .as_ref()
        .map(|r| r.effects.bar_lerp_speed)
        .unwrap_or(8.0);
    let dt = time.delta_seconds();
    anim.current += (anim.target - anim.current) * (1.0 - (-speed * dt).exp());
    style.width = Val::Percent(anim.current * 100.0);
}

pub fn update_health_text(
    player_q: Query<&Health, (With<Player>, With<LocalControlled>)>,
    mut text_q: Query<&mut Text, With<HealthText>>,
) {
    let Ok(hp) = player_q.get_single() else {
        return;
    };
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    text.sections[0].value = format!("HP: {:.0} / {:.0}", hp.current, hp.max);
}

pub fn update_experience_bar(
    player_q: Query<&PlayerLevel, (With<Player>, With<LocalControlled>)>,
    time: Res<Time>,
    registry: Option<Res<GameDataRegistry>>,
    mut fill_q: Query<(&mut Style, &mut BarAnimState), With<ExperienceFill>>,
) {
    let Ok(lvl) = player_q.get_single() else {
        return;
    };
    let Ok((mut style, mut anim)) = fill_q.get_single_mut() else {
        return;
    };
    let ratio = if lvl.xp_to_next > 0 {
        (lvl.xp as f32 / lvl.xp_to_next as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    anim.target = ratio;
    let speed = registry
        .as_ref()
        .map(|r| r.effects.bar_lerp_speed)
        .unwrap_or(8.0);
    let dt = time.delta_seconds();
    anim.current += (anim.target - anim.current) * (1.0 - (-speed * dt).exp());
    style.width = Val::Percent(anim.current * 100.0);
}

pub fn update_experience_text(
    player_q: Query<&PlayerLevel, (With<Player>, With<LocalControlled>)>,
    mut text_q: Query<&mut Text, With<ExperienceText>>,
    mut level_q: Query<&mut Text, (With<LevelText>, Without<ExperienceText>)>,
) {
    let Ok(lvl) = player_q.get_single() else {
        return;
    };
    if let Ok(mut text) = text_q.get_single_mut() {
        text.sections[0].value = format!("{} / {}", lvl.xp, lvl.xp_to_next);
    }
    if let Ok(mut text) = level_q.get_single_mut() {
        text.sections[0].value = format!("Lv.{}", lvl.level);
    }
}

pub fn update_gold_text(
    player_q: Query<&Gold, (With<Player>, With<LocalControlled>)>,
    mut text_q: Query<&mut Text, With<GoldText>>,
) {
    let Ok(gold) = player_q.get_single() else {
        return;
    };
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    text.sections[0].value = format!("金币: {}", gold.0);
}

pub fn update_energy_text(
    player_q: Query<&Energy, (With<Player>, With<LocalControlled>)>,
    time: Res<Time>,
    registry: Option<Res<GameDataRegistry>>,
    mut text_q: Query<&mut Text, With<EnergyText>>,
    mut fill_q: Query<(&mut Style, &mut BackgroundColor, &mut BarAnimState), With<EnergyFill>>,
) {
    let Ok(energy) = player_q.get_single() else {
        return;
    };
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    text.sections[0].value = format!("蓄力: {:.0} / {:.0}", energy.current, energy.max);

    let Ok((mut style, mut color, mut anim)) = fill_q.get_single_mut() else {
        return;
    };
    let ratio = if energy.max > 0.0 {
        (energy.current / energy.max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    anim.target = ratio;
    let speed = registry
        .as_ref()
        .map(|r| r.effects.bar_lerp_speed)
        .unwrap_or(8.0);
    let dt = time.delta_seconds();
    anim.current += (anim.target - anim.current) * (1.0 - (-speed * dt).exp());
    style.width = Val::Percent(anim.current * 100.0);

    let pulse = (time.elapsed_seconds() * 8.0).sin().abs();
    color.0 = if ratio >= 0.999 {
        Color::srgb(0.96, 0.88 + pulse * 0.10, 0.32)
    } else {
        Color::srgb(0.24, 0.72, 0.96)
    };
}

pub fn update_skill_bar_ui(
    time: Res<Time>,
    player_q: Query<
        (&Energy, Option<&SkillSlots>, Option<&PlayerSkillState>),
        (With<Player>, With<LocalControlled>),
    >,
    mut panel_q: Query<(&SkillSlotPanel, &mut BackgroundColor), Without<SkillOverlay>>,
    mut name_q: Query<(&SkillSlotName, &mut Text), Without<SkillSlotKey>>,
    mut key_q: Query<(&SkillSlotKey, &mut Text), Without<SkillSlotName>>,
    mut overlay_q: Query<&mut BackgroundColor, (With<SkillOverlay>, Without<SkillSlotPanel>)>,
) {
    let Ok((energy, slots, skill_state)) = player_q.get_single() else {
        return;
    };
    let slots = slots.copied().unwrap_or_default();
    let energy_ready = energy.current >= energy.max.max(1.0) - f32::EPSILON;
    let pulse = (time.elapsed_seconds() * 7.0).sin().abs();

    if let Ok(mut overlay) = overlay_q.get_single_mut() {
        overlay.0 = if skill_state.is_some_and(PlayerSkillState::lock_on_active) {
            Color::srgba(0.02, 0.03, 0.05, 0.18)
        } else {
            Color::srgba(0.02, 0.03, 0.05, 0.0)
        };
    }

    for (slot_panel, mut bg) in &mut panel_q {
        let state = slots.state(slot_panel.0);
        let base = match state.skill.map(skill_palette) {
            Some(color) => color,
            None => Color::srgb(0.24, 0.24, 0.28),
        };
        bg.0 = if !state.unlocked {
            Color::srgb(0.14, 0.16, 0.20)
        } else if energy_ready {
            state
                .skill
                .map(|skill| skill_ready_palette(skill, pulse))
                .unwrap_or(base)
        } else {
            // Dim the slot when energy not ready — grey tint
            let ratio = (energy.current / energy.max.max(1.0)).clamp(0.0, 1.0);
            let dim = 0.3 + ratio * 0.7;
            let c = base.to_srgba();
            Color::srgba(c.red * dim, c.green * dim, c.blue * dim, c.alpha)
        };
    }

    for (slot_name, mut text) in &mut name_q {
        let state = slots.state(slot_name.0);
        text.sections[0].value = if state.unlocked {
            state
                .skill
                .map(|skill| skill.label())
                .unwrap_or("空槽")
                .to_string()
        } else {
            "锁定".to_string()
        };
        text.sections[0].style.color = if state.unlocked {
            Color::srgb(0.96, 0.97, 0.98)
        } else {
            Color::srgb(0.60, 0.62, 0.66)
        };
    }

    for (slot_key, mut text) in &mut key_q {
        let state = slots.state(slot_key.0);
        text.sections[0].style.color = if !state.unlocked {
            Color::srgb(0.46, 0.48, 0.52)
        } else if energy_ready {
            Color::srgb(1.0, 0.94, 0.54)
        } else {
            Color::srgb(0.80, 0.84, 0.90)
        };
    }
}

pub fn update_dash_cooldown_ui(
    player_q: Query<&DashCooldown, (With<Player>, With<LocalControlled>)>,
    session_q: Query<&CoopSessionState>,
    config: Option<Res<CoopNetConfig>>,
    mut text_q: Query<&mut Text, With<DashText>>,
    mut icon_q: Query<(&mut Style, &mut BackgroundColor), With<DashIconFill>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    let Ok((mut style, mut color)) = icon_q.get_single_mut() else {
        return;
    };

    // client 端从 CoopSessionState 读取 p2 冷却比例（已由 host 同步）
    let is_client = config
        .as_deref()
        .map(|c| c.mode == NetMode::Client)
        .unwrap_or(false);

    if is_client {
        let frac = session_q
            .iter()
            .next()
            .map(|s| s.p2_dash_cooldown_frac)
            .unwrap_or(0.0);
        let ready = frac <= 0.0;
        // frac 是剩余比例（0=就绪，1=刚冲刺完），进度条显示已恢复部分
        let progress = 1.0 - frac;
        style.height = Val::Percent(progress * 100.0);
        *color = BackgroundColor(if ready {
            Color::srgb(0.18, 0.82, 0.45)
        } else {
            Color::srgb(0.95, 0.58, 0.24)
        });
        text.sections[0].value = if ready {
            "冲刺：就绪".to_string()
        } else {
            "冲刺：冷却中".to_string()
        };
        return;
    }

    // host 端或单机模式：直接读本地 DashCooldown
    let Ok(cd) = player_q.get_single() else {
        return;
    };

    let duration = cd.timer.duration().as_secs_f32();
    let remaining = (duration - cd.timer.elapsed_secs()).max(0.0);
    let progress = if duration > 0.0 {
        (cd.timer.elapsed_secs() / duration).clamp(0.0, 1.0)
    } else {
        1.0
    };

    style.height = Val::Percent(progress * 100.0);
    *color = BackgroundColor(if cd.timer.finished() {
        Color::srgb(0.18, 0.82, 0.45)
    } else {
        Color::srgb(0.95, 0.58, 0.24)
    });

    text.sections[0].value = if cd.timer.finished() {
        "冲刺：就绪".to_string()
    } else {
        format!("冲刺：{remaining:.1} 秒")
    };
}

pub fn update_floor_text(
    floor: Option<Res<FloorNumber>>,
    data: Option<Res<GameDataRegistry>>,
    session_q: Query<&CoopSessionState, With<Replicated>>,
    mut text_q: Query<&mut Text, With<FloorText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    let Some(floor) = floor else {
        let total_floors = data
            .as_deref()
            .map(|value| value.balance.total_floors.max(1))
            .unwrap_or(4);
        if let Ok(session) = session_q.get_single() {
            text.sections[0].value = format!(
                "楼层：第 {} 层 / 共 {} 层",
                session.floor_number, total_floors
            );
        }
        return;
    };
    let total_floors = data
        .as_deref()
        .map(|value| value.balance.total_floors.max(1))
        .unwrap_or(4);
    text.sections[0].value = format!("楼层：第 {} 层 / 共 {} 层", floor.0, total_floors);
}

pub fn update_room_text(
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    room_state: Option<Res<RoomState>>,
    session_q: Query<&CoopSessionState, With<Replicated>>,
    mut text_q: Query<&mut Text, With<RoomText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    let (Some(layout), Some(current), Some(room_state)) = (layout, current, room_state) else {
        if let Ok(session) = session_q.get_single() {
            text.sections[0].value = format!(
                "房间：{}（{}）",
                coop_room_type_label(session.room_type),
                coop_room_state_label(session.room_state)
            );
        }
        return;
    };

    let room_type = layout
        .room(current.0)
        .map(|room| room.room_type)
        .unwrap_or(RoomType::Start);
    let room_label = match room_type {
        RoomType::Start => "起始",
        RoomType::Normal => "战斗",
        RoomType::Shop => "商店",
        RoomType::Reward => "奖励",
        RoomType::Event => "事件",
        RoomType::Elite => "精英",
        RoomType::Boss => "首领",
    };
    let state_label = match *room_state {
        RoomState::Idle => "可通行",
        RoomState::Locked => "已封锁",
        RoomState::Cleared => "已清理",
        RoomState::BossFight => "首领战",
    };

    text.sections[0].value = format!("房间：{room_label}（{state_label}）");
}

pub fn update_enemy_count_text(
    config: Option<Res<CoopNetConfig>>,
    authority_enemy_q: Query<(), (With<Enemy>, Without<Replicated>)>,
    replicated_enemy_q: Query<(), (With<Enemy>, With<Replicated>)>,
    mut text_q: Query<&mut Text, With<EnemyCountText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    struct CountProxy(usize);
    impl CountProxy {
        fn iter(&self) -> std::ops::Range<usize> {
            0..self.0
        }
    }
    let enemy_q = CountProxy(
        if config.as_deref().map(|value| value.mode) == Some(NetMode::Client) {
            replicated_enemy_q.iter().count()
        } else {
            authority_enemy_q.iter().count()
        },
    );
    text.sections[0].value = format!("敌人：{}", enemy_q.iter().count());
}

pub fn update_hint_text(
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    room_state: Option<Res<RoomState>>,
    session_q: Query<&CoopSessionState, With<Replicated>>,
    mut text_q: Query<&mut Text, With<HintText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    let (Some(layout), Some(current), Some(room_state)) = (layout, current, room_state) else {
        if let Ok(session) = session_q.get_single() {
            text.sections[0].value =
                coop_hint_text(session.room_type, session.room_state).to_string();
        }
        return;
    };

    let room_type = layout
        .room(current.0)
        .map(|room| room.room_type)
        .unwrap_or(RoomType::Start);
    let hint = match (room_type, *room_state) {
        (RoomType::Start, _) => "提示：长按鼠标左右键持续攻击，靠近门后按 E 前进",
        (RoomType::Reward, _) => "提示：这里会提供奖励，整理状态后继续前进",
        (RoomType::Shop, _) => "提示：商店房可按数字键购买，离开后继续推进",
        (RoomType::Boss, RoomState::BossFight) => "提示：保持移动，合理冲刺，抓住首领空档输出",
        (_, RoomState::Locked) => "提示：清掉房间内所有敌人或完成机关后，门才会开启",
        (_, RoomState::Cleared) => "提示：房门已经打开，靠近后按 E 切换房间",
        _ => "提示：靠近房门后按 E 进入下一个房间",
    };

    text.sections[0].value = hint.to_string();
}

pub fn update_stage_progress(
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    floor: Option<Res<FloorNumber>>,
    session_q: Query<&CoopSessionState, With<Replicated>>,
    mut text_q: Query<&mut Text, With<StageProgressText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    let (Some(layout), Some(current)) = (layout, current) else {
        if let Ok(session) = session_q.get_single() {
            text.sections[0].value = format!(
                "联机进度：第 {} 层 · 房间 {} · {}",
                session.floor_number,
                session.current_room + 1,
                coop_room_type_label(session.room_type)
            );
        }
        return;
    };

    let distances = room_distances_from_start(&layout);
    let current_step = distances.get(&current.0).copied().unwrap_or(0) + 1;
    let total_steps = layout
        .rooms
        .iter()
        .filter(|room| room.room_type == RoomType::Boss)
        .filter_map(|room| distances.get(&room.id).copied())
        .min()
        .unwrap_or_else(|| distances.values().copied().max().unwrap_or(0))
        + 1;
    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);

    text.sections[0].value = format!("第{}层：{}/{}", floor_number, current_step, total_steps);
}

fn room_distances_from_start(layout: &FloorLayout) -> HashMap<RoomId, u32> {
    let mut distances = HashMap::new();
    let mut queue = VecDeque::from([(RoomId(0), 0u32)]);

    while let Some((room_id, distance)) = queue.pop_front() {
        if distances.contains_key(&room_id) {
            continue;
        }
        distances.insert(room_id, distance);

        if let Some(room) = layout.room(room_id) {
            for (_, next_room) in &room.connections.exits {
                if !distances.contains_key(next_room) {
                    queue.push_back((*next_room, distance + 1));
                }
            }
        }
    }

    distances
}

fn skill_palette(skill: crate::gameplay::player::components::SkillType) -> Color {
    use crate::gameplay::player::components::SkillType;
    match skill {
        SkillType::GroundSlam => Color::srgb(0.20, 0.54, 0.40),
        SkillType::BladeDance => Color::srgb(0.28, 0.62, 0.58),
        SkillType::ExecutionBlade => Color::srgb(0.62, 0.22, 0.26),
        SkillType::BulletBarrage => Color::srgb(0.70, 0.48, 0.22),
        SkillType::FrostField => Color::srgb(0.30, 0.66, 0.88),
        SkillType::MeteorFall => Color::srgb(0.78, 0.28, 0.16),
        SkillType::WarCry => Color::srgb(0.72, 0.58, 0.18),
        SkillType::LifeDrain => Color::srgb(0.58, 0.18, 0.30),
        SkillType::TimeRift => Color::srgb(0.42, 0.26, 0.76),
    }
}

fn skill_ready_palette(skill: crate::gameplay::player::components::SkillType, pulse: f32) -> Color {
    use crate::gameplay::player::components::SkillType;
    match skill {
        SkillType::GroundSlam => Color::srgb(0.42, 0.86 + pulse * 0.08, 0.68),
        SkillType::BladeDance => Color::srgb(0.48, 0.92, 0.82),
        SkillType::ExecutionBlade => Color::srgb(0.86 + pulse * 0.08, 0.38, 0.42),
        SkillType::BulletBarrage => Color::srgb(0.94, 0.70, 0.38),
        SkillType::FrostField => Color::srgb(0.56, 0.88, 1.0),
        SkillType::MeteorFall => Color::srgb(1.0, 0.48, 0.28),
        SkillType::WarCry => Color::srgb(0.98, 0.84, 0.36),
        SkillType::LifeDrain => Color::srgb(0.88, 0.34, 0.46),
        SkillType::TimeRift => Color::srgb(0.66, 0.50, 1.0),
    }
}

pub fn update_boss_health_bar(
    config: Option<Res<CoopNetConfig>>,
    authority_boss_q: Query<
        (&crate::gameplay::player::components::Health, &EnemyKind),
        (
            With<crate::gameplay::enemy::components::Enemy>,
            Without<Replicated>,
        ),
    >,
    replicated_boss_q: Query<
        (&crate::gameplay::player::components::Health, &EnemyKind),
        (
            With<crate::gameplay::enemy::components::Enemy>,
            With<Replicated>,
        ),
    >,
    mut boss_fill_q: Query<&mut Style, With<BossHealthFill>>,
    mut boss_bar_q: Query<&mut Visibility, With<BossHealthBar>>,
) {
    let Ok(mut style) = boss_fill_q.get_single_mut() else {
        return;
    };
    let Ok(mut visibility) = boss_bar_q.get_single_mut() else {
        return;
    };

    let boss = if config.as_deref().map(|value| value.mode) == Some(NetMode::Client) {
        replicated_boss_q
            .iter()
            .find_map(|(hp, kind)| (kind.0 == EnemyType::Boss).then_some(hp))
    } else {
        authority_boss_q
            .iter()
            .find_map(|(hp, kind)| (kind.0 == EnemyType::Boss).then_some(hp))
    };
    let Some(hp) = boss else {
        *visibility = Visibility::Hidden;
        return;
    };

    *visibility = Visibility::Visible;
    let ratio = if hp.max > 0.0 {
        (hp.current / hp.max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    style.width = Val::Percent(ratio * 100.0);
}

fn coop_room_type_label(room_type: RoomType) -> &'static str {
    match room_type {
        RoomType::Start => "起始",
        RoomType::Normal => "战斗",
        RoomType::Shop => "商店",
        RoomType::Reward => "奖励",
        RoomType::Event => "事件",
        RoomType::Elite => "精英",
        RoomType::Boss => "首领",
    }
}

fn coop_room_state_label(room_state: RoomState) -> &'static str {
    match room_state {
        RoomState::Idle => "可通行",
        RoomState::Locked => "已封锁",
        RoomState::Cleared => "已清空",
        RoomState::BossFight => "首领战",
    }
}

fn coop_hint_text(room_type: RoomType, room_state: RoomState) -> &'static str {
    match (room_type, room_state) {
        (RoomType::Start, _) => "提示：靠近房门并按 E，和队友一起推进。",
        (RoomType::Reward, _) => "提示：奖励选择会在联机弹窗中显示。",
        (RoomType::Shop, _) => "提示：商店购买与离开操作都在联机弹窗中进行。",
        (RoomType::Boss, RoomState::BossFight) => "提示：保持走位、合理冲刺，抓住首领空档输出。",
        (_, RoomState::Locked) => "提示：先清空房间，房门才会开启。",
        (_, RoomState::Cleared) => "提示：房间已清空，靠近房门并按 E 前进。",
        _ => "提示：尽量和队友保持同步推进。",
    }
}

pub fn cleanup_hud(mut commands: Commands, q: Query<Entity, With<HudUi>>) {
    for entity in &q {
        safe_despawn_recursive(&mut commands, entity);
    }
}
