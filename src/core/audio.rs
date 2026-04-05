use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use bevy_kira_audio::prelude::StaticSoundData;
use rand::Rng;

use crate::core::assets::GameAssets;
use crate::core::events::{SfxEvent, SfxKind};
use crate::data::registry::GameDataRegistry;

const SAMPLE_RATE: u32 = 44100;

// --- Procedural sound generation (pure functions returning PCM f32 samples) ---

fn gen_attack() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.05) as usize;
    let mut buf = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let freq = 300.0 - t * 4000.0; // downward sweep
        let phase = (t * freq * std::f32::consts::TAU).sin();
        let square = if phase > 0.0 { 0.6 } else { -0.6 };
        let env = 1.0 - (i as f32 / len as f32);
        buf.push(square * env);
    }
    buf
}

fn gen_ranged() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.04) as usize;
    let mut buf = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let freq = 600.0 + t * 8000.0; // upward sweep
        let sample = (t * freq * std::f32::consts::TAU).sin() * 0.5;
        let env = 1.0 - (i as f32 / len as f32);
        buf.push(sample * env);
    }
    buf
}

fn gen_dash() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.06) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    for i in 0..len {
        let env = (1.0 - (i as f32 / len as f32)).powi(3);
        let noise: f32 = rng.gen_range(-1.0..1.0);
        buf.push(noise * 0.4 * env);
    }
    buf
}

fn gen_hit() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.03) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = 1.0 - (i as f32 / len as f32);
        let low = (t * 80.0 * std::f32::consts::TAU).sin() * 0.5;
        let noise: f32 = rng.gen_range(-1.0..1.0) * 0.3;
        buf.push((low + noise) * env);
    }
    buf
}

fn gen_crit() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.04) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = 1.0 - (i as f32 / len as f32);
        let low = (t * 80.0 * std::f32::consts::TAU).sin() * 0.5;
        let high = (t * 2400.0 * std::f32::consts::TAU).sin() * 0.3;
        let noise: f32 = rng.gen_range(-1.0..1.0) * 0.2;
        buf.push((low + high + noise) * env);
    }
    buf
}

fn gen_death() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.12) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = 1.0 - (i as f32 / len as f32);
        let mod_freq = 200.0 - t * 1500.0;
        let carrier = (t * mod_freq * std::f32::consts::TAU).sin();
        let noise: f32 = rng.gen_range(-1.0..1.0) * 0.2;
        buf.push((carrier * 0.5 + noise) * env);
    }
    buf
}

fn gen_ui_click() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.015) as usize;
    let mut buf = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = 1.0 - (i as f32 / len as f32);
        buf.push((t * 3200.0 * std::f32::consts::TAU).sin() * 0.4 * env);
    }
    buf
}

fn gen_skill_activate() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.10) as usize;
    let mut buf = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = 1.0 - (i as f32 / len as f32);
        let note = if t < 0.033 { 523.0 } else if t < 0.066 { 659.0 } else { 784.0 };
        buf.push((t * note * std::f32::consts::TAU).sin() * 0.45 * env);
    }
    buf
}

fn gen_room_clear() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.15) as usize;
    let mut buf = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - (i as f32 / len as f32)).sqrt();
        let c = (t * 523.0 * std::f32::consts::TAU).sin();
        let e = (t * 659.0 * std::f32::consts::TAU).sin();
        let g = (t * 784.0 * std::f32::consts::TAU).sin();
        buf.push((c + e + g) / 3.0 * 0.45 * env);
    }
    buf
}

fn gen_boss_phase() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.08) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = 1.0 - (i as f32 / len as f32);
        let low = (t * 60.0 * std::f32::consts::TAU).sin() * 0.6;
        let noise: f32 = rng.gen_range(-1.0..1.0) * 0.3;
        buf.push((low + noise) * env);
    }
    buf
}

fn gen_reward_pickup() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.08) as usize;
    let mut buf = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = 1.0 - (i as f32 / len as f32);
        let freq = 800.0 + t * 2000.0;
        buf.push((t * freq * std::f32::consts::TAU).sin() * 0.35 * env);
    }
    buf
}

fn gen_shop_purchase() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.06) as usize;
    let mut buf = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = 1.0 - (i as f32 / len as f32);
        let note = if t < 0.03 { 880.0 } else { 1100.0 };
        buf.push((t * note * std::f32::consts::TAU).sin() * 0.4 * env);
    }
    buf
}

// --- SFX handles resource ---

fn pcm_to_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = samples.len() as u32;
    let byte_rate = sample_rate * 2;
    let data_size = num_samples * 2;
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(file_size as usize + 8);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let val = (clamped * 32767.0) as i16;
        buf.extend_from_slice(&val.to_le_bytes());
    }
    buf
}

#[derive(Resource, Default)]
pub struct SfxHandles {
    pub melee_attack: Handle<bevy_kira_audio::AudioSource>,
    pub ranged_attack: Handle<bevy_kira_audio::AudioSource>,
    pub dash: Handle<bevy_kira_audio::AudioSource>,
    pub hit: Handle<bevy_kira_audio::AudioSource>,
    pub crit_hit: Handle<bevy_kira_audio::AudioSource>,
    pub enemy_death: Handle<bevy_kira_audio::AudioSource>,
    pub boss_death: Handle<bevy_kira_audio::AudioSource>,
    pub ui_click: Handle<bevy_kira_audio::AudioSource>,
    pub skill_activate: Handle<bevy_kira_audio::AudioSource>,
    pub boss_phase_change: Handle<bevy_kira_audio::AudioSource>,
    pub room_clear: Handle<bevy_kira_audio::AudioSource>,
    pub reward_pickup: Handle<bevy_kira_audio::AudioSource>,
    pub shop_purchase: Handle<bevy_kira_audio::AudioSource>,
}

impl SfxHandles {
    pub fn get(&self, kind: SfxKind) -> &Handle<bevy_kira_audio::AudioSource> {
        match kind {
            SfxKind::MeleeAttack => &self.melee_attack,
            SfxKind::RangedAttack => &self.ranged_attack,
            SfxKind::Dash => &self.dash,
            SfxKind::Hit => &self.hit,
            SfxKind::CritHit => &self.crit_hit,
            SfxKind::EnemyDeath => &self.enemy_death,
            SfxKind::BossDeath => &self.boss_death,
            SfxKind::UiClick => &self.ui_click,
            SfxKind::SkillActivate => &self.skill_activate,
            SfxKind::BossPhaseChange => &self.boss_phase_change,
            SfxKind::RoomClear => &self.room_clear,
            SfxKind::RewardPickup => &self.reward_pickup,
            SfxKind::ShopPurchase => &self.shop_purchase,
        }
    }
}

fn add_wav(
    audio_assets: &mut Assets<bevy_kira_audio::AudioSource>,
    samples: Vec<f32>,
) -> Handle<bevy_kira_audio::AudioSource> {
    let wav_bytes = pcm_to_wav(&samples, SAMPLE_RATE);
    audio_assets.add(bevy_kira_audio::AudioSource { sound: StaticSoundData::from_cursor(std::io::Cursor::new(wav_bytes), StaticSoundSettings::default()).unwrap() })
}

pub fn generate_sfx_assets(
    mut audio_assets: ResMut<Assets<bevy_kira_audio::AudioSource>>,
    mut commands: Commands,
) {
    let handles = SfxHandles {
        melee_attack: add_wav(&mut audio_assets, gen_attack()),
        ranged_attack: add_wav(&mut audio_assets, gen_ranged()),
        dash: add_wav(&mut audio_assets, gen_dash()),
        hit: add_wav(&mut audio_assets, gen_hit()),
        crit_hit: add_wav(&mut audio_assets, gen_crit()),
        enemy_death: add_wav(&mut audio_assets, gen_death()),
        boss_death: add_wav(&mut audio_assets, gen_death()),
        ui_click: add_wav(&mut audio_assets, gen_ui_click()),
        skill_activate: add_wav(&mut audio_assets, gen_skill_activate()),
        boss_phase_change: add_wav(&mut audio_assets, gen_boss_phase()),
        room_clear: add_wav(&mut audio_assets, gen_room_clear()),
        reward_pickup: add_wav(&mut audio_assets, gen_reward_pickup()),
        shop_purchase: add_wav(&mut audio_assets, gen_shop_purchase()),
    };
    commands.insert_resource(handles);
}

// --- Playback system ---

pub fn sfx_playback_system(
    mut events: EventReader<SfxEvent>,
    audio: Res<bevy_kira_audio::Audio>,
    sfx: Option<Res<SfxHandles>>,
    registry: Option<Res<GameDataRegistry>>,
) {
    let Some(sfx) = sfx else { return };
    let volume = registry
        .as_ref()
        .map(|r| (r.audio.master_volume * r.audio.sfx_volume) as f64)
        .unwrap_or(0.56);
    let pitch_var = registry
        .as_ref()
        .map(|r| r.audio.pitch_variation)
        .unwrap_or(0.08);

    let mut rng = rand::thread_rng();
    for ev in events.read() {
        let handle = sfx.get(ev.kind);
        let pitch = 1.0 + rng.gen_range(-pitch_var..pitch_var) as f64;
        audio
            .play(handle.clone())
            .with_volume(Volume::Amplitude(volume))
            .with_playback_rate(pitch);
    }
}

// --- Plugin ---

/// Bridge system: converts existing game events into SfxEvents
pub fn sfx_bridge_system(
    mut damage_events: EventReader<crate::core::events::DamageAppliedEvent>,
    mut room_events: EventReader<crate::core::events::RoomClearedEvent>,
    mut boss_events: EventReader<crate::core::events::BossPhaseChangeEvent>,
    mut sfx_writer: EventWriter<SfxEvent>,
    mut hitstop_writer: EventWriter<crate::core::events::HitStopRequest>,
    mut flash_writer: EventWriter<crate::core::events::ScreenFlashRequest>,
    mut shake_writer: EventWriter<crate::gameplay::effects::screen_shake::ScreenShakeRequest>,
    registry: Option<Res<GameDataRegistry>>,
) {
    let cfg = registry.as_ref().map(|r| &r.effects);
    for ev in damage_events.read() {
        if ev.is_crit {
            sfx_writer.send(SfxEvent { kind: SfxKind::CritHit });
            if let Some(cfg) = cfg {
                hitstop_writer.send(crate::core::events::HitStopRequest { duration_s: cfg.hitstop_crit_s });
            }
        } else {
            sfx_writer.send(SfxEvent { kind: SfxKind::Hit });
            if let Some(cfg) = cfg {
                hitstop_writer.send(crate::core::events::HitStopRequest { duration_s: cfg.hitstop_duration_s });
            }
        }
    }
    for _ev in room_events.read() {
        sfx_writer.send(SfxEvent { kind: SfxKind::RoomClear });
    }
    for _ev in boss_events.read() {
        sfx_writer.send(SfxEvent { kind: SfxKind::BossPhaseChange });
        // Boss phase change: screen flash + shake
        flash_writer.send(crate::core::events::ScreenFlashRequest {
            color: Color::srgba(0.9, 0.2, 0.1, 0.5),
            duration_s: 0.25,
        });
        shake_writer.send(crate::gameplay::effects::screen_shake::ScreenShakeRequest {
            strength: 8.0,
            duration: 0.3,
        });
        hitstop_writer.send(crate::core::events::HitStopRequest { duration_s: 0.1 });
    }
}

// --- BGM system framework ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgmTrack {
    None,
    Menu,
    Exploration,
    Combat,
    Boss,
}

#[derive(Resource)]
pub struct BgmState {
    pub current: BgmTrack,
    pub target: BgmTrack,
    pub fade_timer: Timer,
    pub volume: f32,
}

impl Default for BgmState {
    fn default() -> Self {
        Self {
            current: BgmTrack::None,
            target: BgmTrack::None,
            fade_timer: Timer::from_seconds(1.0, TimerMode::Once),
            volume: 0.0,
        }
    }
}

impl BgmState {
    pub fn request(&mut self, track: BgmTrack) {
        if self.target != track {
            self.target = track;
            self.fade_timer.reset();
        }
    }
}

pub fn bgm_state_sync_system(
    state: Res<State<crate::states::AppState>>,
    room_state: Option<Res<crate::states::RoomState>>,
    mut bgm: ResMut<BgmState>,
) {
    use crate::states::AppState;
    let track = match state.get() {
        AppState::MainMenu => BgmTrack::Menu,
        AppState::InGame | AppState::CoopGame => {
            if room_state.as_deref() == Some(&crate::states::RoomState::BossFight) {
                BgmTrack::Boss
            } else if room_state.as_deref() == Some(&crate::states::RoomState::Locked) {
                BgmTrack::Combat
            } else {
                BgmTrack::Exploration
            }
        }
        _ => BgmTrack::None,
    };
    bgm.request(track);
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BgmState>()
            .add_systems(Startup, generate_sfx_assets)
            .add_systems(Update, (sfx_playback_system, sfx_bridge_system, bgm_state_sync_system));
    }
}
