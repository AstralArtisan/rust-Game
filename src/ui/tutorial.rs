use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;
use lightyear::prelude::Replicated;
use serde::{Deserialize, Serialize};

use crate::core::assets::GameAssets;
use crate::gameplay::enemy::components::{Elite, Enemy};
use crate::gameplay::map::room::{CurrentRoom, FloorLayout, RoomType};
use crate::gameplay::player::components::{Energy, Player, SkillSlot};
use crate::gameplay::skills::SkillUnlockedEvent;
use crate::ui::widgets;
use crate::utils::entity::safe_despawn_recursive;

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TutorialFlags {
    pub movement_hint_shown: bool,
    pub charge_ready_hint_shown: bool,
    pub shop_hint_shown: bool,
    pub elite_hint_shown: bool,
    pub unlock_hints_shown: [bool; 4],
}

impl Default for TutorialFlags {
    fn default() -> Self {
        Self {
            movement_hint_shown: false,
            charge_ready_hint_shown: false,
            shop_hint_shown: false,
            elite_hint_shown: false,
            unlock_hints_shown: [false; 4],
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct TutorialBannerQueue {
    pub pending: VecDeque<String>,
    pub active: Option<ActiveBanner>,
}

#[derive(Debug)]
pub struct ActiveBanner {
    pub text: String,
    pub timer: Timer,
}

#[derive(Component)]
pub struct TutorialUi;

#[derive(Component)]
pub struct TutorialBanner;

#[derive(Component)]
pub struct TutorialBannerText;

pub struct TutorialPlugin;

impl Plugin for TutorialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TutorialFlags>()
            .init_resource::<TutorialBannerQueue>()
            .add_systems(Startup, load_tutorial_flags)
            .add_systems(OnEnter(crate::states::AppState::InGame), setup_tutorial_ui)
            .add_systems(
                OnEnter(crate::states::AppState::InGame),
                queue_initial_movement_hint,
            )
            .add_systems(OnExit(crate::states::AppState::InGame), cleanup_tutorial_ui)
            .add_systems(
                Update,
                (
                    queue_charge_ready_hint,
                    queue_shop_hint,
                    queue_elite_hint,
                    queue_skill_unlock_hints,
                    update_tutorial_banner,
                    persist_tutorial_flags,
                )
                    .run_if(in_state(crate::states::AppState::InGame)),
            );
    }
}

pub fn setup_tutorial_ui(mut commands: Commands, assets: Res<GameAssets>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(24.0),
                    left: Val::Percent(25.0),
                    width: Val::Percent(50.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            },
            TutorialUi,
            Name::new("TutorialUi"),
        ))
        .with_children(|root| {
            root.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.0)),
                    ..default()
                },
                TutorialBanner,
            ))
            .with_children(|banner| {
                banner.spawn((widgets::title_text(&assets, "", 18.0), TutorialBannerText));
            });
        });
}

pub fn cleanup_tutorial_ui(mut commands: Commands, q: Query<Entity, With<TutorialUi>>) {
    for entity in &q {
        safe_despawn_recursive(&mut commands, entity);
    }
}

pub fn queue_initial_movement_hint(
    mut flags: ResMut<TutorialFlags>,
    mut queue: ResMut<TutorialBannerQueue>,
) {
    if flags.movement_hint_shown {
        return;
    }
    enqueue_banner(
        &mut queue,
        "WASD 移动 / 左键近战 / 右键远程 / 空格冲刺".to_string(),
    );
    flags.movement_hint_shown = true;
}

pub fn queue_charge_ready_hint(
    mut flags: ResMut<TutorialFlags>,
    mut queue: ResMut<TutorialBannerQueue>,
    player_q: Query<&Energy, (With<Player>, Without<Replicated>)>,
) {
    if flags.charge_ready_hint_shown {
        return;
    }
    let Ok(energy) = player_q.get_single() else {
        return;
    };
    if energy.current + f32::EPSILON < energy.max {
        return;
    }
    enqueue_banner(&mut queue, "蓄力已满！按 1/2/3 释放终结技".to_string());
    flags.charge_ready_hint_shown = true;
}

pub fn queue_shop_hint(
    layout: Option<Res<FloorLayout>>,
    current: Option<Res<CurrentRoom>>,
    mut flags: ResMut<TutorialFlags>,
    mut queue: ResMut<TutorialBannerQueue>,
) {
    if flags.shop_hint_shown {
        return;
    }
    let (Some(layout), Some(current)) = (layout, current) else {
        return;
    };
    if layout.room(current.0).map(|room| room.room_type) != Some(RoomType::Shop) {
        return;
    }
    enqueue_banner(&mut queue, "按 E 打开商店".to_string());
    flags.shop_hint_shown = true;
}

pub fn queue_elite_hint(
    elite_q: Query<(), (With<Elite>, With<Enemy>, Without<Replicated>)>,
    mut flags: ResMut<TutorialFlags>,
    mut queue: ResMut<TutorialBannerQueue>,
) {
    if flags.elite_hint_shown || elite_q.is_empty() {
        return;
    }
    enqueue_banner(&mut queue, "精英敌人！击杀获得大量蓄力能量".to_string());
    flags.elite_hint_shown = true;
}

pub fn queue_skill_unlock_hints(
    mut events: EventReader<SkillUnlockedEvent>,
    mut flags: ResMut<TutorialFlags>,
    mut queue: ResMut<TutorialBannerQueue>,
) {
    for event in events.read() {
        let idx = event.slot.index();
        if flags.unlock_hints_shown[idx] {
            continue;
        }
        let name = match event.slot {
            SkillSlot::One => "剑气斩",
            SkillSlot::Two => "标记猎杀",
            SkillSlot::Three => "闪电冲刺",
            SkillSlot::Four => "遗物主动",
        };
        enqueue_banner(
            &mut queue,
            format!("已解锁 {name}！按 {} 释放", event.slot.key_label()),
        );
        flags.unlock_hints_shown[idx] = true;
    }
}

pub fn update_tutorial_banner(
    time: Res<Time>,
    mut queue: ResMut<TutorialBannerQueue>,
    mut banner_q: Query<&mut BackgroundColor, With<TutorialBanner>>,
    mut text_q: Query<&mut Text, With<TutorialBannerText>>,
) {
    let Ok(mut background) = banner_q.get_single_mut() else {
        return;
    };
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };

    if queue.active.is_none() {
        if let Some(next) = queue.pending.pop_front() {
            queue.active = Some(ActiveBanner {
                text: next,
                timer: Timer::from_seconds(3.2, TimerMode::Once),
            });
        }
    }

    let Some(active) = queue.active.as_mut() else {
        background.0 = Color::srgba(0.06, 0.08, 0.12, 0.0);
        text.sections[0].value.clear();
        return;
    };

    active.timer.tick(time.delta());
    let t = active.timer.fraction();
    let alpha = if t < 0.12 {
        t / 0.12
    } else if t > 0.82 {
        (1.0 - t) / 0.18
    } else {
        1.0
    }
    .clamp(0.0, 1.0);

    background.0 = Color::srgba(0.06, 0.08, 0.12, 0.72 * alpha);
    text.sections[0].value = active.text.clone();
    text.sections[0].style.color = Color::srgba(0.96, 0.97, 0.98, alpha);

    if active.timer.finished() {
        queue.active = None;
    }
}

pub fn persist_tutorial_flags(flags: Res<TutorialFlags>) {
    if !flags.is_changed() {
        return;
    }
    let path = tutorial_flags_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let pretty = ron::ser::PrettyConfig::new().depth_limit(3);
    if let Ok(serialized) = ron::ser::to_string_pretty(&*flags, pretty) {
        let _ = fs::write(path, serialized);
    }
}

fn load_tutorial_flags(mut flags: ResMut<TutorialFlags>) {
    let path = tutorial_flags_path();
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    if let Ok(saved) = ron::from_str::<TutorialFlags>(&content) {
        *flags = saved;
    }
}

fn tutorial_flags_path() -> PathBuf {
    PathBuf::from("saves").join("tutorial_flags.ron")
}

fn enqueue_banner(queue: &mut TutorialBannerQueue, text: String) {
    queue.pending.push_back(text);
}
