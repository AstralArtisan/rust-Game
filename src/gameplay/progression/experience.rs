use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::events::RoomClearedEvent;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::player::components::Player;
use crate::states::AppState;
use crate::ui::levelup_select::{LevelUpChoices, LevelUpOption, LevelUpStat};
use crate::utils::rng::GameRng;

/// Tracks player level and XP within a run.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct PlayerLevel {
    pub level: u32,
    pub xp: u32,
    pub xp_to_next: u32,
}

impl Default for PlayerLevel {
    fn default() -> Self {
        Self {
            level: 1,
            xp: 0,
            xp_to_next: 40,
        }
    }
}

impl PlayerLevel {
    /// Add XP and return the number of levels gained.
    pub fn add_xp(&mut self, amount: u32) -> u32 {
        self.xp += amount;
        let mut levels_gained = 0u32;
        while self.xp >= self.xp_to_next {
            self.xp -= self.xp_to_next;
            self.level += 1;
            levels_gained += 1;
            self.xp_to_next = Self::xp_threshold(self.level);
        }
        levels_gained
    }

    /// XP needed to go from `level` to `level+1`.
    pub fn xp_threshold(level: u32) -> u32 {
        40 + (level.saturating_sub(1)) * 15
    }
}

#[derive(Event, Debug, Clone, Copy)]
pub struct XpGainEvent {
    pub amount: u32,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct LevelUpEvent {
    pub new_level: u32,
}

/// System: processes XP gain events, updates PlayerLevel, emits LevelUpEvent.
pub fn process_xp_gains(
    mut xp_events: EventReader<XpGainEvent>,
    mut levelup_events: EventWriter<LevelUpEvent>,
    mut player_q: Query<(&mut PlayerLevel, Option<&AugmentInventory>), With<Player>>,
) {
    let total_xp: u32 = xp_events.read().map(|e| e.amount).sum();
    if total_xp == 0 {
        return;
    }
    for (mut level, inventory) in &mut player_q {
        let xp_mult = match inventory
            .map(|value| value.stacks(AugmentId::XpBonus))
            .unwrap_or(0)
        {
            2 => 1.50,
            1 => 1.25,
            _ => 1.0,
        };
        let adjusted_xp = (total_xp as f32 * xp_mult) as u32;
        let levels_gained = level.add_xp(adjusted_xp);
        for i in 0..levels_gained {
            levelup_events.send(LevelUpEvent {
                new_level: level.level - levels_gained + i + 1,
            });
        }
    }
}

/// Resource: queues level-up events so they don't race with room-clear rewards.
#[derive(Resource, Debug, Default)]
pub struct PendingLevelUps {
    pub levels: Vec<u32>,
}

/// System: when a LevelUpEvent fires, generate 3 random stat options and enter LevelUpSelect.
/// Defers if a RoomClearedEvent is pending in the same frame (Boss kill gives XP + room clear).
pub fn handle_levelup_event(
    mut levelup_events: EventReader<LevelUpEvent>,
    mut pending: ResMut<PendingLevelUps>,
    mut choices: ResMut<LevelUpChoices>,
    mut next_state: ResMut<NextState<AppState>>,
    mut rng: ResMut<GameRng>,
    current_state: Res<State<AppState>>,
    room_cleared: EventReader<RoomClearedEvent>,
) {
    // Collect new level-ups into pending queue
    for ev in levelup_events.read() {
        pending.levels.push(ev.new_level);
    }

    if pending.levels.is_empty() {
        return;
    }

    // Don't pop level-up if a room clear is happening this frame (let rewards go first)
    if !room_cleared.is_empty() {
        return;
    }

    // Only trigger when we're actually in a gameplay state
    let return_state = match current_state.get() {
        AppState::InGame => AppState::InGame,
        AppState::CoopGame => AppState::CoopGame,
        _ => return, // Not in gameplay, wait
    };

    let new_level = pending.levels.remove(0);

    let all_stats: Vec<(LevelUpStat, &str, &str)> = vec![
        (LevelUpStat::AttackPower(3.0), "攻击力 +3", "提升近战和远程攻击伤害"),
        (LevelUpStat::MaxHealth(15.0), "生命上限 +15", "提升最大生命值并回复等量HP"),
        (LevelUpStat::MoveSpeed(15.0), "移动速度 +15", "提升角色移动速度"),
        (LevelUpStat::CritChance(0.05), "暴击率 +5%", "提升暴击概率"),
        (LevelUpStat::AttackSpeed(0.05), "攻速 +0.05s", "缩短攻击冷却时间"),
        (LevelUpStat::DashCooldown(0.1), "冲刺冷却 -0.1s", "缩短冲刺冷却时间"),
    ];

    let mut indices: Vec<usize> = (0..all_stats.len()).collect();
    rng.shuffle(&mut indices);
    indices.truncate(3);

    choices.options = indices
        .iter()
        .map(|&i| {
            let (stat, label, desc) = &all_stats[i];
            LevelUpOption {
                label: label.to_string(),
                description: desc.to_string(),
                apply: *stat,
            }
        })
        .collect();
    choices.return_state = Some(return_state);
    choices.new_level = new_level;

    next_state.set(AppState::LevelUpSelect);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_level() {
        let level = PlayerLevel::default();
        assert_eq!(level.level, 1);
        assert_eq!(level.xp, 0);
        assert_eq!(level.xp_to_next, 40);
    }

    #[test]
    fn test_add_xp_no_levelup() {
        let mut level = PlayerLevel::default();
        let gained = level.add_xp(30);
        assert_eq!(gained, 0);
        assert_eq!(level.level, 1);
        assert_eq!(level.xp, 30);
    }

    #[test]
    fn test_add_xp_levelup() {
        let mut level = PlayerLevel::default();
        let gained = level.add_xp(45);
        assert_eq!(gained, 1);
        assert_eq!(level.level, 2);
        assert_eq!(level.xp, 5);
        assert_eq!(level.xp_to_next, 55);
    }

    #[test]
    fn test_multi_levelup() {
        let mut level = PlayerLevel::default();
        // Level 1→2: 40 XP, Level 2→3: 55 XP, Level 3→4: 70 XP = 165 total
        let gained = level.add_xp(200);
        assert_eq!(gained, 3);
        assert_eq!(level.level, 4);
        assert_eq!(level.xp, 200 - 40 - 55 - 70); // 35
        assert_eq!(level.xp_to_next, 85);
    }

    #[test]
    fn test_xp_threshold_formula() {
        assert_eq!(PlayerLevel::xp_threshold(1), 40);
        assert_eq!(PlayerLevel::xp_threshold(2), 55);
        assert_eq!(PlayerLevel::xp_threshold(5), 100);
    }

    #[test]
    fn test_pending_levelups_default() {
        let pending = PendingLevelUps::default();
        assert!(pending.levels.is_empty());
    }
}
