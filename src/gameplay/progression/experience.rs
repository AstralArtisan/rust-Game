use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
    mut player_q: Query<&mut PlayerLevel, With<crate::gameplay::player::components::Player>>,
) {
    let total_xp: u32 = xp_events.read().map(|e| e.amount).sum();
    if total_xp == 0 {
        return;
    }
    for mut level in &mut player_q {
        let levels_gained = level.add_xp(total_xp);
        for i in 0..levels_gained {
            levelup_events.send(LevelUpEvent {
                new_level: level.level - levels_gained + i + 1,
            });
        }
    }
}
