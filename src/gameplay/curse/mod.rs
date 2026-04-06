#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::gameplay::map::room::{CurrentRoom, RoomId};
use crate::gameplay::player::components::Player;
use crate::states::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CurseId {
    Fragile,
    Sluggish,
    Exhaustion,
    Exposed,
    Weakness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveCurse {
    pub curse: CurseId,
    pub rooms_remaining: u32,
}

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurseState {
    pub active: Vec<ActiveCurse>,
}

impl CurseState {
    pub fn has_any_curse(&self) -> bool {
        !self.active.is_empty()
    }

    pub fn add_curse(&mut self, curse: CurseId, duration: u32) {
        self.active.push(ActiveCurse {
            curse,
            rooms_remaining: duration,
        });
    }

    pub fn tick_room(&mut self) -> Vec<CurseId> {
        let mut expired = Vec::new();
        self.active.retain_mut(|curse| {
            curse.rooms_remaining = curse.rooms_remaining.saturating_sub(1);
            if curse.rooms_remaining == 0 {
                expired.push(curse.curse);
                false
            } else {
                true
            }
        });
        expired
    }

    #[allow(dead_code)]
    pub fn damage_taken_mult(&self) -> f32 {
        let mut mult = 1.0;
        for curse in &self.active {
            if curse.curse == CurseId::Fragile {
                mult *= 1.25;
            }
        }
        mult
    }

    #[allow(dead_code)]
    pub fn move_speed_mult(&self) -> f32 {
        let mut mult = 1.0;
        for curse in &self.active {
            if curse.curse == CurseId::Sluggish {
                mult *= 0.80;
            }
        }
        mult
    }

    #[allow(dead_code)]
    pub fn energy_gain_mult(&self) -> f32 {
        let mut mult = 1.0;
        for curse in &self.active {
            if curse.curse == CurseId::Exhaustion {
                mult *= 0.60;
            }
        }
        mult
    }

    #[allow(dead_code)]
    pub fn dash_cooldown_mult(&self) -> f32 {
        let mut mult = 1.0;
        for curse in &self.active {
            if curse.curse == CurseId::Exposed {
                mult *= 1.50;
            }
        }
        mult
    }

    #[allow(dead_code)]
    pub fn damage_dealt_mult(&self) -> f32 {
        let mut mult = 1.0;
        for curse in &self.active {
            if curse.curse == CurseId::Weakness {
                mult *= 0.80;
            }
        }
        mult
    }
}

pub struct CursePlugin;

impl Plugin for CursePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            tick_curses_on_room_change
                .run_if(in_state(AppState::InGame).or_else(in_state(AppState::CoopGame))),
        );
    }
}

fn tick_curses_on_room_change(
    current: Option<Res<CurrentRoom>>,
    mut last_room: Local<Option<RoomId>>,
    mut player_q: Query<&mut CurseState, With<Player>>,
) {
    let Some(current) = current else {
        *last_room = None;
        return;
    };

    let room_changed = match *last_room {
        Some(previous) => previous != current.0,
        None => false,
    };

    if room_changed {
        for mut curse_state in &mut player_q {
            curse_state.tick_room();
        }
    }

    *last_room = Some(current.0);
}
