use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use bevy_kira_audio::prelude::StaticSoundData;
use rand::Rng;

use crate::core::assets::GameAssets;
use crate::core::events::{SfxEvent, SfxKind};
use crate::data::registry::GameDataRegistry;

const SAMPLE_RATE: u32 = 44100;
const DT: f32 = 1.0 / SAMPLE_RATE as f32;
const TAU: f32 = std::f32::consts::TAU;

// --- Helpers ---

/// Advance phase accumulator and return sin. Correct for frequency sweeps.
fn osc(phase: &mut f32, freq: f32) -> f32 {
    *phase += freq * DT * TAU;
    phase.sin()
}

/// Square wave via phase accumulator.
fn osc_sq(phase: &mut f32, freq: f32) -> f32 {
    *phase += freq * DT * TAU;
    if phase.sin() > 0.0 { 1.0 } else { -1.0 }
}

/// Linear interpolation.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// --- Procedural sound generation ---

fn gen_attack() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.055) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    let mut phase = 0.0_f32;
    for i in 0..len {
        let t = i as f32 / len as f32;
        let env = (1.0 - t).powf(1.5);
        // Layer 1: square wave downsweep 280→80 Hz
        let freq = lerp(280.0, 80.0, t);
        let sq = osc_sq(&mut phase, freq) * 0.45;
        // Layer 2: noise transient in first 10ms
        let noise_env = if i < (SAMPLE_RATE as f32 * 0.01) as usize {
            (1.0 - i as f32 / (SAMPLE_RATE as f32 * 0.01)).powi(2)
        } else {
            0.0
        };
        let noise = rng.gen_range(-1.0..1.0_f32) * 0.35 * noise_env;
        buf.push((sq + noise) * env);
    }
    buf
}

fn gen_ranged() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.045) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut phase1 = 0.0_f32;
    let mut phase2 = 0.0_f32;
    for i in 0..len {
        let t = i as f32 / len as f32;
        let env = (1.0 - t).powi(2);
        // Layer 1: sine upsweep 500→1400 Hz
        let freq = lerp(500.0, 1400.0, t);
        let sine = osc(&mut phase1, freq) * 0.4;
        // Layer 2: high-freq "pew" onset in first 8ms
        let onset_env = if i < (SAMPLE_RATE as f32 * 0.008) as usize {
            1.0 - i as f32 / (SAMPLE_RATE as f32 * 0.008)
        } else {
            0.0
        };
        let onset = osc(&mut phase2, 2200.0) * 0.25 * onset_env;
        buf.push((sine + onset) * env);
    }
    buf
}

fn gen_dash() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.07) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    let mut phase = 0.0_f32;
    for i in 0..len {
        let t = i as f32 / len as f32;
        let env = (1.0 - t).powi(3);
        // Layer 1: filtered noise (multiply noise by mid-freq sine to band-pass)
        let noise: f32 = rng.gen_range(-1.0..1.0);
        let filter = (i as f32 * DT * 800.0 * TAU).sin();
        let filtered = noise * filter.abs() * 0.3;
        // Layer 2: low sine downsweep 200→60 Hz ("whoosh" body)
        let freq = lerp(200.0, 60.0, t);
        let low = osc(&mut phase, freq) * 0.25;
        buf.push((filtered + low) * env);
    }
    buf
}

fn gen_hit() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.04) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    let mut ph1 = 0.0_f32;
    let mut ph2 = 0.0_f32;
    let onset_len = (SAMPLE_RATE as f32 * 0.005) as usize;
    for i in 0..len {
        let t = i as f32 / len as f32;
        let env = (-8.0 * t).exp();
        // Layer 1: low thump 110 Hz
        let low = osc(&mut ph1, 110.0) * 0.5;
        // Layer 2: noise transient first 5ms
        let noise_env = if i < onset_len { (1.0 - i as f32 / onset_len as f32).powi(2) } else { 0.0 };
        let noise = rng.gen_range(-1.0..1.0_f32) * 0.4 * noise_env;
        // Layer 3: mid-freq body 350 Hz
        let mid = osc(&mut ph2, 350.0) * 0.2 * (-12.0 * t).exp();
        buf.push((low + noise + mid) * env);
    }
    buf
}

fn gen_crit() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.055) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    let mut ph1 = 0.0_f32;
    let mut ph2 = 0.0_f32;
    let mut ph3 = 0.0_f32;
    let mut ph4 = 0.0_f32;
    let onset_len = (SAMPLE_RATE as f32 * 0.005) as usize;
    for i in 0..len {
        let t = i as f32 / len as f32;
        let env = (-6.0 * t).exp();
        // Layers 1-3: same as hit but louder
        let low = osc(&mut ph1, 110.0) * 0.55;
        let noise_env = if i < onset_len { (1.0 - i as f32 / onset_len as f32).powi(2) } else { 0.0 };
        let noise = rng.gen_range(-1.0..1.0_f32) * 0.45 * noise_env;
        let mid = osc(&mut ph2, 350.0) * 0.25 * (-10.0 * t).exp();
        // Layer 4: metallic high sweep 2000→1600 Hz
        let hi_freq = lerp(2000.0, 1600.0, t);
        let hi = osc(&mut ph3, hi_freq) * 0.2 * (-10.0 * t).exp();
        // Layer 5: sub-bass pulse 55 Hz, first 15ms
        let sub_env = if i < (SAMPLE_RATE as f32 * 0.015) as usize {
            1.0 - i as f32 / (SAMPLE_RATE as f32 * 0.015)
        } else { 0.0 };
        let sub = osc(&mut ph4, 55.0) * 0.3 * sub_env;
        buf.push((low + noise + mid + hi + sub) * env);
    }
    buf
}

fn gen_death() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.14) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    let mut ph1 = 0.0_f32;
    let mut ph2 = 0.0_f32;
    let onset_len = (SAMPLE_RATE as f32 * 0.03) as usize;
    for i in 0..len {
        let t = i as f32 / len as f32;
        let env = (1.0 - t).powf(0.7);
        // Layer 1: FM downsweep 220→40 Hz, modulation depth increases
        let freq = lerp(220.0, 40.0, t);
        let mod_depth = lerp(0.5, 3.0, t);
        let modulator = osc(&mut ph2, freq * 1.5) * mod_depth;
        let carrier = osc(&mut ph1, freq + modulator * freq);
        let l1 = carrier * 0.4;
        // Layer 2: noise increasing 30%→60%
        let noise_amt = lerp(0.15, 0.35, t);
        let noise = rng.gen_range(-1.0..1.0_f32) * noise_amt;
        // Layer 3: high overtone 440 Hz, fast decay first 30ms
        let hi_env = if i < onset_len { (1.0 - i as f32 / onset_len as f32).powi(2) } else { 0.0 };
        let hi = (i as f32 * DT * 440.0 * TAU).sin() * 0.2 * hi_env;
        buf.push((l1 + noise + hi) * env);
    }
    buf
}

fn gen_ui_click() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.018) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut ph1 = 0.0_f32;
    let mut ph2 = 0.0_f32;
    for i in 0..len {
        let t = i as f32 / len as f32;
        let env = (-20.0 * t).exp();
        let main = osc(&mut ph1, 1800.0) * 0.35;
        let overtone = osc(&mut ph2, 2700.0) * 0.12;
        buf.push((main + overtone) * env);
    }
    buf
}

fn gen_skill_activate() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.12) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    let mut ph1 = 0.0_f32;
    let mut ph2 = 0.0_f32;
    let third = len / 3;
    for i in 0..len {
        let t = i as f32 / len as f32;
        let env = (1.0 - t).powf(0.8);
        // Arpeggio: C5→E5→G5 with smooth phase accumulation
        let note = if i < third { 523.0 } else if i < third * 2 { 659.0 } else { 784.0 };
        let main = osc(&mut ph1, note) * 0.35;
        // Octave overtone
        let oct = osc(&mut ph2, note * 2.0) * 0.1;
        // Noise sweep layer (energy release)
        let noise = rng.gen_range(-1.0..1.0_f32) * 0.08 * t;
        buf.push((main + oct + noise) * env);
    }
    buf
}

fn gen_room_clear() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.20) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut ph_c = 0.0_f32;
    let mut ph_e = 0.0_f32;
    let mut ph_g = 0.0_f32;
    let mut ph_c2 = 0.0_f32;
    for i in 0..len {
        let t = i as f32 / len as f32;
        let env = (1.0 - t).sqrt();
        // C major chord + octave C
        let c = osc(&mut ph_c, 523.0) * 0.25;
        let e = osc(&mut ph_e, 659.0) * 0.2;
        let g = osc(&mut ph_g, 784.0) * 0.2;
        let c2 = osc(&mut ph_c2, 1046.0) * 0.12;
        // Simple reverb simulation: mix in delayed copy
        let dry = c + e + g + c2;
        buf.push(dry * env);
    }
    // Add simple echo/reverb pass
    let delay = (SAMPLE_RATE as f32 * 0.03) as usize;
    for i in delay..buf.len() {
        buf[i] += buf[i - delay] * 0.2;
    }
    buf
}

fn gen_boss_phase() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.15) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    let mut ph1 = 0.0_f32;
    let mut ph2 = 0.0_f32;
    for i in 0..len {
        let t = i as f32 / len as f32;
        // Trapezoid envelope: ramp up 30%, sustain 30%, decay 40%
        let env = if t < 0.3 {
            t / 0.3
        } else if t < 0.6 {
            1.0
        } else {
            (1.0 - t) / 0.4
        };
        // Layer 1: sub-bass drone 45 Hz
        let drone = osc(&mut ph1, 45.0) * 0.5;
        // Layer 2: dissonant minor second 47.5 Hz
        let dissonant = osc(&mut ph2, 47.5) * 0.35;
        // Layer 3: noise crescendo
        let noise_amt = lerp(0.05, 0.35, t);
        let noise = rng.gen_range(-1.0..1.0_f32) * noise_amt;
        buf.push((drone + dissonant + noise) * env);
    }
    buf
}

fn gen_reward_pickup() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.10) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut ph1 = 0.0_f32;
    let mut ph2 = 0.0_f32;
    let half = len / 2;
    for i in 0..len {
        let env_local = if i < half {
            (-10.0 * (i as f32 / half as f32)).exp()
        } else {
            (-10.0 * ((i - half) as f32 / half as f32)).exp()
        };
        // Two-note rise: C5 then E5
        let note = if i < half { 523.0 } else { 659.0 };
        let main = osc(&mut ph1, note) * 0.35;
        // Shimmer layer
        let shimmer = osc(&mut ph2, 3000.0) * 0.08 * (-15.0 * (i as f32 / len as f32)).exp();
        buf.push((main + shimmer) * env_local);
    }
    buf
}

fn gen_shop_purchase() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.08) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut ph1 = 0.0_f32;
    let mut ph2 = 0.0_f32;
    let half = len / 2;
    for i in 0..len {
        let env_local = if i < half {
            (-12.0 * (i as f32 / half as f32)).exp()
        } else {
            (-12.0 * ((i - half) as f32 / half as f32)).exp()
        };
        // Two-note confirm: G5 → C6
        let note = if i < half { 784.0 } else { 1047.0 };
        let main = osc(&mut ph1, note) * 0.3;
        let overtone = osc(&mut ph2, note * 2.0) * 0.08;
        buf.push((main + overtone) * env_local);
    }
    buf
}

fn gen_boss_death() -> Vec<f32> {
    let len = (SAMPLE_RATE as f32 * 0.35) as usize;
    let mut buf = Vec::with_capacity(len);
    let mut rng = rand::thread_rng();
    let mut ph1 = 0.0_f32;
    let mut ph2 = 0.0_f32;
    let onset_len = (SAMPLE_RATE as f32 * 0.05) as usize;
    for i in 0..len {
        let t = i as f32 / len as f32;
        // Envelope: burst then long tail
        let env = if t < 0.1 { 1.0 } else { (-3.0 * (t - 0.1)).exp() };
        // Layer 1: deep downsweep 180→25 Hz
        let freq = lerp(180.0, 25.0, t);
        let sweep = osc(&mut ph1, freq) * 0.45;
        // Layer 2: noise burst + slow decay
        let noise = rng.gen_range(-1.0..1.0_f32) * 0.3 * (-4.0 * t).exp();
        // Layer 3: low rumble 30 Hz throughout
        let rumble = osc(&mut ph2, 30.0) * 0.25;
        // Layer 4: high shatter 1200 Hz, fast decay first 50ms
        let hi_env = if i < onset_len { (1.0 - i as f32 / onset_len as f32).powi(2) } else { 0.0 };
        let shatter = (i as f32 * DT * 1200.0 * TAU).sin() * 0.2 * hi_env;
        buf.push((sweep + noise + rumble + shatter) * env);
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
        boss_death: add_wav(&mut audio_assets, gen_boss_death()),
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
        app.add_plugins(bevy_kira_audio::AudioPlugin)
            .init_resource::<BgmState>()
            .add_systems(Startup, generate_sfx_assets)
            .add_systems(Update, (sfx_playback_system, sfx_bridge_system, bgm_state_sync_system));
    }
}
