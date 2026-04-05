use bevy::prelude::*;

use crate::gameplay::combat::components::{DamageKind, Team};
use crate::gameplay::map::room::RoomId;
use crate::gameplay::rewards::data::RewardType;

#[derive(Event, Debug, Clone, Copy)]
pub struct DamageEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub amount: f32,
    pub knockback: Vec2,
    pub team: Team,
    pub kind: DamageKind,
    pub is_crit: bool,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct DamageAppliedEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub amount: f32,
    pub attacker_team: Team,
    pub kind: DamageKind,
    pub target_team: Option<Team>,
    pub is_crit: bool,
    pub pos: Vec2,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct DeathEvent {
    pub entity: Entity,
    pub team: Team,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct RoomClearedEvent {
    pub room: RoomId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardChoiceGroup {
    Heal,
    Primary,
    Secondary,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct RewardChosenEvent {
    pub reward: RewardType,
    pub group: RewardChoiceGroup,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct DoorOpenEvent {
    pub room: RoomId,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct SpawnEnemyEvent {
    pub room: RoomId,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct BossPhaseChangeEvent {
    pub phase: u8,
}

// --- 音效 / 视觉反馈事件 ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SfxKind {
    MeleeAttack,
    RangedAttack,
    Dash,
    Hit,
    CritHit,
    EnemyDeath,
    BossDeath,
    UiClick,
    SkillActivate,
    BossPhaseChange,
    RoomClear,
    RewardPickup,
    ShopPurchase,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct SfxEvent {
    pub kind: SfxKind,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct HitStopRequest {
    pub duration_s: f32,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct ScreenFlashRequest {
    pub color: Color,
    pub duration_s: f32,
}

pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DamageEvent>()
            .add_event::<DamageAppliedEvent>()
            .add_event::<DeathEvent>()
            .add_event::<RoomClearedEvent>()
            .add_event::<RewardChosenEvent>()
            .add_event::<DoorOpenEvent>()
            .add_event::<SpawnEnemyEvent>()
            .add_event::<BossPhaseChangeEvent>()
            .add_event::<SfxEvent>()
            .add_event::<HitStopRequest>()
            .add_event::<ScreenFlashRequest>();
    }
}
