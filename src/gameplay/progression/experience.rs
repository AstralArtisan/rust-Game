use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::events::RoomClearedEvent;
use crate::data::definitions::{LevelUpConfig, RewardScalingConfig};
use crate::data::registry::GameDataRegistry;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::augment::tuning;
use crate::gameplay::player::components::{Health, Player};
use crate::gameplay::progression::floor::FloorNumber;
use crate::gameplay::rewards::apply::heal_amount;
use crate::states::{AppState, GamePhase};
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
            xp_to_next: xp_threshold(&[], 1),
        }
    }
}

impl PlayerLevel {
    /// Add XP and return the number of levels gained.
    pub fn add_xp(&mut self, amount: u32, curve: &[u32]) -> u32 {
        self.xp += amount;
        let mut levels_gained = 0u32;
        while self.xp >= self.xp_to_next {
            self.xp -= self.xp_to_next;
            self.level += 1;
            levels_gained += 1;
            self.xp_to_next = xp_threshold(curve, self.level);
        }
        levels_gained
    }
}

/// XP needed to go from `level` to `level+1`. Reads from `EconomyConfig::xp_curve`;
/// extrapolates linearly (+25 / level) beyond the configured table, and falls
/// back to a hard-coded baseline of `[50, 70, 90, …]` when the curve is empty.
pub fn xp_threshold(curve: &[u32], level: u32) -> u32 {
    const FALLBACK: [u32; 9] = [50, 70, 90, 110, 130, 150, 180, 200, 220];
    let table: &[u32] = if curve.is_empty() { &FALLBACK } else { curve };
    let idx = level.saturating_sub(1) as usize;
    if let Some(&v) = table.get(idx) {
        v
    } else if let Some(&last) = table.last() {
        last + ((idx + 1).saturating_sub(table.len())) as u32 * 25
    } else {
        50
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
    data: Res<GameDataRegistry>,
    mut xp_events: EventReader<XpGainEvent>,
    mut levelup_events: EventWriter<LevelUpEvent>,
    mut player_q: Query<(&mut PlayerLevel, Option<&AugmentInventory>), With<Player>>,
) {
    let total_xp: u32 = xp_events.read().map(|e| e.amount).sum();
    if total_xp == 0 {
        return;
    }
    for (mut level, inventory) in &mut player_q {
        let xp_mult = tuning::xp_bonus_mult(
            &data,
            inventory
                .map(|value| value.stacks(AugmentId::XpBonus))
                .unwrap_or(0),
        );
        let adjusted_xp = (total_xp as f32 * xp_mult) as u32;
        let levels_gained = level.add_xp(adjusted_xp, &data.economy.xp_curve);
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

pub fn build_levelup_options(
    rng: &mut GameRng,
    scaling: &RewardScalingConfig,
    levelup: &LevelUpConfig,
    max_health: f32,
    floor_number: u32,
) -> Vec<LevelUpOption> {
    let heal_value = heal_amount(scaling, max_health, floor_number);
    let all_stats: Vec<(LevelUpStat, String, &str)> = vec![
        (
            LevelUpStat::AttackPower(levelup.attack_power),
            format!("攻击力 +{:.0}", levelup.attack_power),
            "提升近战和远程攻击伤害",
        ),
        (
            LevelUpStat::MaxHealth(levelup.max_health),
            format!("生命上限 +{:.0}", levelup.max_health),
            "提升最大生命值并回复等量 HP",
        ),
        (
            LevelUpStat::MoveSpeed(levelup.move_speed),
            format!("移动速度 +{:.0}", levelup.move_speed),
            "提升角色移动速度",
        ),
        (
            LevelUpStat::CritChance(levelup.crit_chance),
            format!("暴击率 +{:.0}%", levelup.crit_chance * 100.0),
            "提升暴击概率",
        ),
        (
            LevelUpStat::MeleeSpeed(levelup.melee_speed_s),
            format!("近战间隔 -{:.2}s", levelup.melee_speed_s),
            "缩短近战攻击冷却",
        ),
        (
            LevelUpStat::RangedSpeed(levelup.ranged_speed_s),
            format!("远程间隔 -{:.2}s", levelup.ranged_speed_s),
            "缩短远程攻击冷却",
        ),
        (
            LevelUpStat::DashCooldown(levelup.dash_cooldown_s),
            format!("冲刺冷却 -{:.2}s", levelup.dash_cooldown_s),
            "缩短冲刺冷却时间",
        ),
    ];

    let mut indices: Vec<usize> = (0..all_stats.len()).collect();
    rng.shuffle(&mut indices);
    indices.truncate(3);

    let mut options = Vec::with_capacity(4);
    options.push(LevelUpOption {
        label: "回血".to_string(),
        description: format!("恢复 {:.0} 生命\n稳住当前状态后继续推进", heal_value),
        apply: LevelUpStat::RecoverHealth(heal_value),
    });
    options.extend(indices.iter().map(|&i| {
        let (stat, label, desc) = &all_stats[i];
        LevelUpOption {
            label: label.clone(),
            description: desc.to_string(),
            apply: *stat,
        }
    }));
    options
}

/// System: when a LevelUpEvent fires, generate 3 random stat options and enter LevelUpSelect.
/// Defers if a RoomClearedEvent is pending in the same frame (Boss kill gives XP + room clear).
pub fn handle_levelup_event(
    mut levelup_events: EventReader<LevelUpEvent>,
    mut pending: ResMut<PendingLevelUps>,
    mut choices: ResMut<LevelUpChoices>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut rng: ResMut<GameRng>,
    current_state: Res<State<AppState>>,
    room_cleared: EventReader<RoomClearedEvent>,
    health_q: Query<&Health, With<Player>>,
    floor: Option<Res<FloorNumber>>,
    data: Option<Res<GameDataRegistry>>,
) {
    for ev in levelup_events.read() {
        pending.levels.push(ev.new_level);
    }

    if pending.levels.is_empty() {
        return;
    }

    if !room_cleared.is_empty() {
        return;
    }

    match current_state.get() {
        AppState::InGame | AppState::CoopGame => {}
        _ => return,
    }
    let return_state = GamePhase::Playing;

    let new_level = pending.levels.remove(0);
    let max_health = health_q
        .get_single()
        .map(|health| health.max)
        .unwrap_or(100.0);
    let floor_number = floor.as_deref().map(|value| value.0).unwrap_or(1);
    let default_scaling = RewardScalingConfig::default_config();
    let default_levelup = LevelUpConfig::default_config();
    let (scaling, levelup) = data
        .as_ref()
        .map(|d| (&d.rewards.scaling, &d.rewards.levelup))
        .unwrap_or((&default_scaling, &default_levelup));

    choices.options = build_levelup_options(&mut rng, scaling, levelup, max_health, floor_number);
    choices.return_state = Some(return_state);
    choices.new_level = new_level;
    choices.crit_cap = levelup.crit_cap;
    choices.melee_min_s = levelup.melee_min_s;
    choices.ranged_min_s = levelup.ranged_min_s;
    choices.dash_min_s = levelup.dash_min_s;

    next_state.set(GamePhase::LevelUpSelect);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_level() {
        let level = PlayerLevel::default();
        assert_eq!(level.level, 1);
        assert_eq!(level.xp, 0);
        assert_eq!(level.xp_to_next, 50);
    }

    const CURVE: &[u32] = &[50, 70, 90, 110, 130, 150, 180, 200, 220];

    #[test]
    fn test_add_xp_no_levelup() {
        let mut level = PlayerLevel::default();
        let gained = level.add_xp(20, CURVE);
        assert_eq!(gained, 0);
        assert_eq!(level.level, 1);
        assert_eq!(level.xp, 20);
    }

    #[test]
    fn test_add_xp_levelup() {
        let mut level = PlayerLevel::default();
        let gained = level.add_xp(50, CURVE);
        assert_eq!(gained, 1);
        assert_eq!(level.level, 2);
        assert_eq!(level.xp, 0);
        assert_eq!(level.xp_to_next, 70);
    }

    #[test]
    fn test_multi_levelup() {
        let mut level = PlayerLevel::default();
        let gained = level.add_xp(200, CURVE);
        assert_eq!(gained, 2);
        assert_eq!(level.level, 3);
        assert_eq!(level.xp, 80);
        assert_eq!(level.xp_to_next, 90);
    }

    #[test]
    fn test_xp_threshold_formula() {
        assert_eq!(xp_threshold(CURVE, 1), 50);
        assert_eq!(xp_threshold(CURVE, 2), 70);
        assert_eq!(xp_threshold(CURVE, 3), 90);
        assert_eq!(xp_threshold(CURVE, 4), 110);
        assert_eq!(xp_threshold(CURVE, 5), 130);
        // Beyond the configured table: 220 + (level - 9) * 25
        assert_eq!(xp_threshold(CURVE, 10), 245);
    }

    #[test]
    fn test_pending_levelups_default() {
        let pending = PendingLevelUps::default();
        assert!(pending.levels.is_empty());
    }

    #[test]
    fn test_build_levelup_options_always_starts_with_heal() {
        let mut rng = GameRng::default();
        rng.reseed(7);

        let options = build_levelup_options(
            &mut rng,
            &RewardScalingConfig::default_config(),
            &LevelUpConfig::default_config(),
            100.0,
            1,
        );

        assert_eq!(options.len(), 4);
        assert!(matches!(options[0].apply, LevelUpStat::RecoverHealth(_)));
    }
}
