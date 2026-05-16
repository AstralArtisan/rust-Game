use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::core::events::RoomClearedEvent;
use crate::data::definitions::{PuzzleEventConfig, PuzzleRewardPool};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::map::room::{CurrentRoom, RoomId};
use crate::gameplay::player::components::Player;
use crate::states::{AppState, GamePhase, RoomState};

pub struct PuzzlePlugin;

impl Plugin for PuzzlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActivePuzzle>().add_systems(
            Update,
            (
                bullet_maze_system,
                memory_blocks_system,
                timed_collect_system,
                complete_active_puzzle_system,
            )
                .run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))),
        );
    }
}

#[derive(Component)]
pub struct PuzzleEntity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PuzzleKind {
    BulletMaze,
    MemoryBlocks,
    TimedCollect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PuzzleReward {
    pub gold: u32,
    pub xp: u32,
    pub augment_pool: PuzzleRewardPool,
}

impl Default for PuzzleReward {
    fn default() -> Self {
        Self {
            gold: 0,
            xp: 0,
            augment_pool: PuzzleRewardPool::None,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct ActivePuzzle {
    pub room: Option<RoomId>,
    pub kind: Option<PuzzleKind>,
    pub completed: bool,
    pub reward_earned: bool,
    pub reward: PuzzleReward,
    target_count: u32,
    progress_count: u32,
    lives_remaining: u32,
    timer: Option<Timer>,
    memory_sequence: Vec<u8>,
    memory_index: usize,
    memory_round: u32,
}

impl Default for ActivePuzzle {
    fn default() -> Self {
        Self {
            room: None,
            kind: None,
            completed: false,
            reward_earned: false,
            reward: PuzzleReward::default(),
            target_count: 0,
            progress_count: 0,
            lives_remaining: 0,
            timer: None,
            memory_sequence: Vec::new(),
            memory_index: 0,
            memory_round: 1,
        }
    }
}

impl ActivePuzzle {
    pub fn progress_count(&self) -> u32 {
        self.progress_count
    }

    pub fn target_count(&self) -> u32 {
        self.target_count
    }

    pub fn reward_to_apply(&self) -> PuzzleReward {
        if !self.reward_earned {
            return PuzzleReward::default();
        }
        if self.kind != Some(PuzzleKind::TimedCollect) {
            return self.reward;
        }

        let target = self.target_count.max(1);
        let collected = self.progress_count.min(target);
        if collected == 0 {
            return PuzzleReward::default();
        }

        let fraction = collected as f32 / target as f32;
        PuzzleReward {
            gold: scale_reward_amount(self.reward.gold, fraction),
            xp: scale_reward_amount(self.reward.xp, fraction),
            augment_pool: if collected >= target {
                self.reward.augment_pool
            } else {
                PuzzleRewardPool::None
            },
        }
    }
}

fn scale_reward_amount(amount: u32, fraction: f32) -> u32 {
    if amount == 0 {
        0
    } else {
        ((amount as f32 * fraction).round() as u32).max(1)
    }
}

#[allow(dead_code)]
pub fn clear_active_puzzle(mut active: ResMut<ActivePuzzle>) {
    reset_active_puzzle(&mut active);
}

pub fn reset_active_puzzle(active: &mut ActivePuzzle) {
    *active = ActivePuzzle::default();
}

pub fn spawn_puzzle_for_kind(
    commands: &mut Commands,
    assets: &GameAssets,
    active: &mut ActivePuzzle,
    room: RoomId,
    kind: PuzzleKind,
    config: &PuzzleEventConfig,
) {
    reset_active_puzzle(active);
    active.room = Some(room);
    active.kind = Some(kind);
    active.target_count = config.target_count.max(1);
    active.lives_remaining = config.lives.max(1);
    active.reward = PuzzleReward {
        gold: config.gold_reward,
        xp: config.xp_reward,
        augment_pool: config.augment_pool,
    };
    active.timer = (config.time_limit_s > 0.0)
        .then(|| Timer::from_seconds(config.time_limit_s, TimerMode::Once));

    match kind {
        PuzzleKind::BulletMaze => spawn_bullet_maze(commands, assets),
        PuzzleKind::MemoryBlocks => {
            active.memory_sequence = (1..=active.target_count.min(7))
                .map(|value| value as u8)
                .collect();
            active.memory_round = 1;
            spawn_memory_blocks(commands, assets, active.memory_sequence.as_slice());
        }
        PuzzleKind::TimedCollect => spawn_collectibles(commands, assets, active.target_count),
    }
}

#[derive(Component)]
struct BulletMazeGoal {
    radius: f32,
}

#[derive(Component)]
struct BulletMazeHazard {
    min_x: f32,
    max_x: f32,
    speed: f32,
    radius: f32,
    dir: f32,
}

#[derive(Component)]
struct MemoryBlock {
    index: u8,
    radius: f32,
}

#[derive(Component)]
struct TimedCollectible {
    radius: f32,
}

fn spawn_bullet_maze(commands: &mut Commands, assets: &GameAssets) {
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(Vec3::new(250.0, 0.0, 40.0)),
            sprite: Sprite {
                color: Color::srgb(0.22, 0.82, 0.44),
                custom_size: Some(Vec2::new(48.0, 90.0)),
                ..default()
            },
            ..default()
        },
        BulletMazeGoal { radius: 58.0 },
        PuzzleEntity,
        InGameEntity,
        Name::new("BulletMazeGoal"),
    ));

    for (i, y) in [-110.0, 0.0, 110.0].into_iter().enumerate() {
        commands.spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(Vec3::new(
                    -140.0 + i as f32 * 90.0,
                    y,
                    45.0,
                )),
                sprite: Sprite {
                    color: Color::srgb(0.90, 0.28, 0.32),
                    custom_size: Some(Vec2::splat(34.0)),
                    ..default()
                },
                ..default()
            },
            BulletMazeHazard {
                min_x: -220.0,
                max_x: 160.0,
                speed: 120.0 + i as f32 * 28.0,
                radius: 38.0,
                dir: if i % 2 == 0 { 1.0 } else { -1.0 },
            },
            PuzzleEntity,
            InGameEntity,
            Name::new("BulletMazeHazard"),
        ));
    }

    spawn_hint(
        commands,
        assets,
        "弹幕迷宫：抵达右侧绿色终点，被击中会回到起点。",
    );
}

fn spawn_memory_blocks(commands: &mut Commands, assets: &GameAssets, sequence: &[u8]) {
    let positions = [
        Vec2::new(-150.0, 80.0),
        Vec2::new(-50.0, -60.0),
        Vec2::new(50.0, 80.0),
        Vec2::new(150.0, -60.0),
        Vec2::new(-210.0, -120.0),
        Vec2::new(0.0, 150.0),
        Vec2::new(210.0, -120.0),
    ];
    for (i, index) in sequence.iter().copied().enumerate() {
        let pos = positions.get(i).copied().unwrap_or(Vec2::ZERO);
        commands.spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(pos.extend(40.0)),
                sprite: Sprite {
                    color: Color::srgb(0.86, 0.74, 0.26),
                    custom_size: Some(Vec2::splat(44.0)),
                    ..default()
                },
                ..default()
            },
            MemoryBlock {
                index,
                radius: 54.0,
            },
            PuzzleEntity,
            InGameEntity,
            Name::new("MemoryBlock"),
        ));
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    format!("{index}"),
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 22.0,
                        color: Color::BLACK,
                    },
                ),
                transform: Transform::from_translation((pos + Vec2::new(0.0, -4.0)).extend(41.0)),
                ..default()
            },
            PuzzleEntity,
            InGameEntity,
            Name::new("MemoryBlockLabel"),
        ));
    }
    spawn_hint(commands, assets, "记忆方块：按显示顺序靠近方块并按 E。");
}

fn spawn_collectibles(commands: &mut Commands, assets: &GameAssets, count: u32) {
    let positions = [
        Vec2::new(-180.0, 90.0),
        Vec2::new(-90.0, -95.0),
        Vec2::new(20.0, 120.0),
        Vec2::new(120.0, -70.0),
        Vec2::new(210.0, 80.0),
        Vec2::new(0.0, -150.0),
        Vec2::new(-230.0, -40.0),
    ];
    for pos in positions.into_iter().take(count as usize) {
        commands.spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(pos.extend(40.0)),
                sprite: Sprite {
                    color: Color::srgb(0.30, 0.82, 1.0),
                    custom_size: Some(Vec2::splat(30.0)),
                    ..default()
                },
                ..default()
            },
            TimedCollectible { radius: 42.0 },
            PuzzleEntity,
            InGameEntity,
            Name::new("TimedCollectible"),
        ));
    }
    spawn_hint(
        commands,
        assets,
        "限时收集：时间结束前收集所有蓝色能量方块。",
    );
}

fn spawn_hint(commands: &mut Commands, assets: &GameAssets, text: &'static str) {
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                text,
                TextStyle {
                    font: assets.font.clone(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            )
            .with_justify(JustifyText::Center),
            transform: Transform::from_translation(Vec3::new(0.0, -178.0, 46.0)),
            ..default()
        },
        PuzzleEntity,
        InGameEntity,
        Name::new("PuzzleHint"),
    ));
}

fn bullet_maze_system(
    time: Res<Time>,
    room_state: Res<RoomState>,
    current_room: Option<Res<CurrentRoom>>,
    mut active: ResMut<ActivePuzzle>,
    mut player_q: Query<&mut Transform, With<Player>>,
    goal_q: Query<(&GlobalTransform, &BulletMazeGoal)>,
    mut hazard_q: Query<(&mut Transform, &mut BulletMazeHazard), Without<Player>>,
) {
    if !puzzle_active(
        &room_state,
        current_room.as_deref(),
        &active,
        PuzzleKind::BulletMaze,
    ) {
        return;
    }
    tick_timer(&time, &mut active);
    if active.completed {
        return;
    }

    let Ok(mut player_tf) = player_q.get_single_mut() else {
        return;
    };
    let player_pos = player_tf.translation.truncate();
    for (goal_tf, goal) in &goal_q {
        if player_pos.distance(goal_tf.translation().truncate()) <= goal.radius {
            active.completed = true;
            active.reward_earned = true;
            return;
        }
    }

    for (mut hazard_tf, mut hazard) in &mut hazard_q {
        hazard_tf.translation.x += hazard.speed * hazard.dir * time.delta_seconds();
        if hazard_tf.translation.x > hazard.max_x {
            hazard_tf.translation.x = hazard.max_x;
            hazard.dir = -1.0;
        } else if hazard_tf.translation.x < hazard.min_x {
            hazard_tf.translation.x = hazard.min_x;
            hazard.dir = 1.0;
        }

        if player_pos.distance(hazard_tf.translation.truncate()) <= hazard.radius {
            active.lives_remaining = active.lives_remaining.saturating_sub(1);
            player_tf.translation = Vec3::new(-250.0, 0.0, player_tf.translation.z);
            if active.lives_remaining == 0 {
                active.completed = true;
                active.reward_earned = false;
            }
            return;
        }
    }
}

fn memory_blocks_system(
    input: Res<crate::core::input::PlayerInputState>,
    room_state: Res<RoomState>,
    current_room: Option<Res<CurrentRoom>>,
    mut active: ResMut<ActivePuzzle>,
    player_q: Query<&GlobalTransform, With<Player>>,
    mut blocks_q: Query<(&GlobalTransform, &MemoryBlock, &mut Sprite)>,
) {
    if !puzzle_active(
        &room_state,
        current_room.as_deref(),
        &active,
        PuzzleKind::MemoryBlocks,
    ) {
        return;
    }
    if active.completed || !input.interact_pressed {
        return;
    }

    let Ok(player_tf) = player_q.get_single() else {
        return;
    };
    let player_pos = player_tf.translation().truncate();
    let round_len = memory_round_len(&active);
    let expected = active.memory_sequence.get(active.memory_index).copied();
    let mut handled_input = false;
    let mut reset_colors = false;

    for (tf, block, mut sprite) in &mut blocks_q {
        if player_pos.distance(tf.translation().truncate()) > block.radius {
            continue;
        }
        handled_input = true;
        if Some(block.index) == expected {
            sprite.color = Color::srgb(0.28, 0.92, 0.36);
            active.memory_index += 1;
            if active.memory_index >= round_len {
                active.progress_count = active.memory_round;
                if active.memory_round >= 3 {
                    active.completed = true;
                    active.reward_earned = true;
                } else {
                    active.memory_round += 1;
                    active.memory_index = 0;
                    reset_colors = true;
                }
            }
            if active.memory_sequence.is_empty() {
                active.completed = true;
                active.reward_earned = true;
            }
        } else {
            active.memory_index = 0;
            active.progress_count = 0;
            active.memory_round = 1;
            reset_colors = true;
            active.lives_remaining = active.lives_remaining.saturating_sub(1);
            if active.lives_remaining == 0 {
                active.completed = true;
                active.reward_earned = false;
            }
        }
        break;
    }

    if reset_colors {
        for (_, _, mut sprite) in &mut blocks_q {
            sprite.color = Color::srgb(0.86, 0.74, 0.26);
        }
    }
    if handled_input {}
}

fn memory_round_len(active: &ActivePuzzle) -> usize {
    let max_len = active
        .target_count
        .min(active.memory_sequence.len() as u32)
        .max(1);
    let start_len = max_len.saturating_sub(2).max(1);
    (start_len + active.memory_round.saturating_sub(1)).min(max_len) as usize
}

fn timed_collect_system(
    mut commands: Commands,
    time: Res<Time>,
    room_state: Res<RoomState>,
    current_room: Option<Res<CurrentRoom>>,
    mut active: ResMut<ActivePuzzle>,
    player_q: Query<&GlobalTransform, With<Player>>,
    collect_q: Query<(Entity, &GlobalTransform, &TimedCollectible)>,
) {
    if !puzzle_active(
        &room_state,
        current_room.as_deref(),
        &active,
        PuzzleKind::TimedCollect,
    ) {
        return;
    }
    tick_timer(&time, &mut active);
    if active.completed {
        return;
    }

    let Ok(player_tf) = player_q.get_single() else {
        return;
    };
    let player_pos = player_tf.translation().truncate();
    for (entity, tf, collectible) in &collect_q {
        if player_pos.distance(tf.translation().truncate()) <= collectible.radius {
            commands.entity(entity).despawn_recursive();
            active.progress_count += 1;
            if active.progress_count >= active.target_count {
                active.completed = true;
                active.reward_earned = true;
            }
            return;
        }
    }
}

fn complete_active_puzzle_system(
    mut commands: Commands,
    current_room: Option<Res<CurrentRoom>>,
    mut room_state: ResMut<RoomState>,
    mut active: ResMut<ActivePuzzle>,
    mut cleared: EventWriter<RoomClearedEvent>,
    puzzle_entities: Query<Entity, With<PuzzleEntity>>,
) {
    let Some(current_room) = current_room else {
        return;
    };
    if !active.completed || active.room != Some(current_room.0) {
        return;
    }
    if !matches!(*room_state, RoomState::Locked) {
        return;
    }

    for entity in &puzzle_entities {
        commands.entity(entity).despawn_recursive();
    }
    *room_state = RoomState::Cleared;
    active.completed = false;
    cleared.send(RoomClearedEvent {
        room: current_room.0,
    });
}

fn puzzle_active(
    room_state: &RoomState,
    current_room: Option<&CurrentRoom>,
    active: &ActivePuzzle,
    kind: PuzzleKind,
) -> bool {
    matches!(*room_state, RoomState::Locked)
        && current_room.is_some_and(|current| active.room == Some(current.0))
        && active.kind == Some(kind)
}

fn tick_timer(time: &Time, active: &mut ActivePuzzle) {
    let Some(timer) = active.timer.as_mut() else {
        return;
    };
    timer.tick(time.delta());
    if timer.finished() {
        active.completed = true;
        active.reward_earned =
            active.kind == Some(PuzzleKind::TimedCollect) && active.progress_count > 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_puzzle_for_kind_maps_config_to_reward_and_state() {
        let mut active = ActivePuzzle::default();
        let config = PuzzleEventConfig {
            time_limit_s: 20.0,
            target_count: 5,
            lives: 3,
            gold_reward: 30,
            xp_reward: 25,
            augment_pool: PuzzleRewardPool::Elite,
        };

        reset_active_puzzle(&mut active);
        active.room = Some(RoomId(7));
        active.kind = Some(PuzzleKind::TimedCollect);
        active.target_count = config.target_count;
        active.lives_remaining = config.lives;
        active.reward = PuzzleReward {
            gold: config.gold_reward,
            xp: config.xp_reward,
            augment_pool: config.augment_pool,
        };

        assert_eq!(active.room, Some(RoomId(7)));
        assert_eq!(active.kind, Some(PuzzleKind::TimedCollect));
        assert_eq!(active.reward.gold, 30);
        assert_eq!(active.reward.xp, 25);
        assert_eq!(active.reward.augment_pool, PuzzleRewardPool::Elite);
    }

    #[test]
    fn bullet_maze_system_queries_do_not_conflict() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ActivePuzzle>()
            .insert_resource(RoomState::Cleared)
            .add_systems(Update, bullet_maze_system);

        app.update();
    }

    #[test]
    fn timed_collect_reward_scales_by_collected_count() {
        let mut active = ActivePuzzle {
            kind: Some(PuzzleKind::TimedCollect),
            reward_earned: true,
            reward: PuzzleReward {
                gold: 25,
                xp: 25,
                augment_pool: PuzzleRewardPool::Any,
            },
            target_count: 5,
            progress_count: 3,
            ..Default::default()
        };

        let reward = active.reward_to_apply();

        assert_eq!(reward.gold, 15);
        assert_eq!(reward.xp, 15);
        assert_eq!(reward.augment_pool, PuzzleRewardPool::None);

        active.progress_count = 5;
        assert_eq!(active.reward_to_apply().augment_pool, PuzzleRewardPool::Any);
    }
}
