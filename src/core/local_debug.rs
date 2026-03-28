use std::env;

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowPosition};

use crate::coop::net::{
    begin_coop_lobby_session, normalize_coop_host_ip, reset_coop_network, CoopNetConfig,
    CoopNetState, CoopSessionFlow, NetMode as CoopNetMode,
};
use crate::gameplay::enemy::systems::EnemySpawnCount;
use crate::gameplay::progression::floor::FloorNumber;
use crate::pvp::net::{
    start_client_socket as start_pvp_client_socket, start_host_socket as start_pvp_host_socket,
    reset_pvp_network, NetMode as PvpNetMode, PvpNetConfig, PvpNetState, PVP_PORT,
};
use crate::states::AppState;

pub struct LocalDebugPlugin;

impl Plugin for LocalDebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LocalNetDebugConfig::from_env())
            .init_resource::<LocalNetDebugRuntime>()
            .add_systems(Update, (apply_local_debug_window, auto_boot_local_debug));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalNetDebugMode {
    Coop,
    Pvp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalNetDebugRole {
    Host,
    Client,
}

#[derive(Resource, Debug, Clone)]
pub struct LocalNetDebugConfig {
    pub enabled: bool,
    pub mode: Option<LocalNetDebugMode>,
    pub role: Option<LocalNetDebugRole>,
    pub host_ip: String,
    pub save_suffix: Option<String>,
    pub title_suffix: Option<String>,
    pub window_pos: Option<IVec2>,
}

impl Default for LocalNetDebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: None,
            role: None,
            host_ip: "127.0.0.1".to_string(),
            save_suffix: None,
            title_suffix: None,
            window_pos: None,
        }
    }
}

impl LocalNetDebugConfig {
    pub fn from_env() -> Self {
        #[cfg(not(debug_assertions))]
        {
            Self::default()
        }

        #[cfg(debug_assertions)]
        {
            let raw_mode = env::var("LOCAL_NET_DEBUG_MODE").ok();
            let raw_role = env::var("LOCAL_NET_DEBUG_ROLE").ok();
            let enabled = env_flag("LOCAL_NET_DEBUG") || raw_mode.is_some() || raw_role.is_some();
            if !enabled {
                return Self::default();
            }

            let mode = raw_mode
                .as_deref()
                .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                    "coop" => Some(LocalNetDebugMode::Coop),
                    "pvp" => Some(LocalNetDebugMode::Pvp),
                    _ => None,
                });
            let role = raw_role
                .as_deref()
                .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                    "host" => Some(LocalNetDebugRole::Host),
                    "client" => Some(LocalNetDebugRole::Client),
                    _ => None,
                });

            if mode.is_none() || role.is_none() {
                return Self::default();
            }

            let mode = mode.unwrap();
            let role = role.unwrap();
            let host_ip = env::var("LOCAL_NET_DEBUG_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            let mode_label = match mode {
                LocalNetDebugMode::Coop => "Coop",
                LocalNetDebugMode::Pvp => "PVP",
            };
            let role_label = match role {
                LocalNetDebugRole::Host => "Host",
                LocalNetDebugRole::Client => "Client",
            };
            let save_suffix = env::var("LOCAL_NET_DEBUG_SAVE_SUFFIX")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| {
                    Some(format!(
                        "{}_{}",
                        mode_label.to_ascii_lowercase(),
                        role_label.to_ascii_lowercase()
                    ))
                });

            Self {
                enabled: true,
                mode: Some(mode),
                role: Some(role),
                host_ip,
                save_suffix,
                title_suffix: Some(format!("[{mode_label} {role_label} Debug]")),
                window_pos: Some(match role {
                    LocalNetDebugRole::Host => IVec2::new(40, 40),
                    LocalNetDebugRole::Client => IVec2::new(980, 40),
                }),
            }
        }
    }

    pub fn save_filename(&self) -> Option<String> {
        if !self.enabled {
            return None;
        }
        self.save_suffix
            .as_ref()
            .map(|suffix| format!("run_save_debug_{suffix}.ron"))
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)]
struct LocalNetDebugRuntime {
    bootstrapped: bool,
    window_applied: bool,
}

pub fn debug_save_filename() -> Option<String> {
    LocalNetDebugConfig::from_env().save_filename()
}

fn apply_local_debug_window(
    config: Res<LocalNetDebugConfig>,
    mut runtime: ResMut<LocalNetDebugRuntime>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !config.enabled || runtime.window_applied {
        return;
    }

    let Ok(mut window) = window_q.get_single_mut() else {
        return;
    };

    if let Some(suffix) = config.title_suffix.as_ref() {
        if !window.title.contains(suffix) {
            window.title = format!("{} {}", window.title, suffix);
        }
    }
    if let Some(pos) = config.window_pos {
        window.position = WindowPosition::At(pos);
    }
    runtime.window_applied = true;
}

fn auto_boot_local_debug(
    mut commands: Commands,
    state: Res<State<AppState>>,
    config: Res<LocalNetDebugConfig>,
    mut runtime: ResMut<LocalNetDebugRuntime>,
    mut coop_config: ResMut<CoopNetConfig>,
    mut coop_net: ResMut<CoopNetState>,
    mut coop_flow: ResMut<CoopSessionFlow>,
    mut pvp_config: ResMut<PvpNetConfig>,
    mut pvp_net: ResMut<PvpNetState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if !config.enabled || runtime.bootstrapped || *state.get() != AppState::MainMenu {
        return;
    }

    reset_coop_network(&mut coop_config, &mut coop_net);
    reset_pvp_network(&mut pvp_config, &mut pvp_net);

    let Some(mode) = config.mode else {
        return;
    };
    let Some(role) = config.role else {
        return;
    };

    let boot_ok = match (mode, role) {
        (LocalNetDebugMode::Coop, LocalNetDebugRole::Host) => {
            coop_config.mode = CoopNetMode::Host;
            coop_config.host_ip.clear();
            if let Err(err) = begin_coop_lobby_session(&coop_config, &mut coop_net, &mut coop_flow)
            {
                warn!("Local coop debug host startup failed: {err:?}");
                false
            } else {
                commands.insert_resource(FloorNumber(1));
                commands.insert_resource(EnemySpawnCount { current: 0 });
                next_state.set(AppState::CoopLobby);
                true
            }
        }
        (LocalNetDebugMode::Coop, LocalNetDebugRole::Client) => {
            match normalize_coop_host_ip(&config.host_ip) {
                Ok(host_ip) => {
                    coop_config.mode = CoopNetMode::Client;
                    coop_config.host_ip = host_ip;
                    if let Err(err) =
                        begin_coop_lobby_session(&coop_config, &mut coop_net, &mut coop_flow)
                    {
                        warn!("Local coop debug client startup failed: {err:?}");
                        false
                    } else {
                        next_state.set(AppState::CoopLobby);
                        true
                    }
                }
                Err(err) => {
                    warn!("Invalid local coop debug host address: {err}");
                    false
                }
            }
        }
        (LocalNetDebugMode::Pvp, LocalNetDebugRole::Host) => {
            pvp_config.mode = PvpNetMode::Host;
            if let Err(err) = start_pvp_host_socket(&mut pvp_net) {
                warn!("Local pvp debug host startup failed: {err:?}");
                false
            } else {
                next_state.set(AppState::PvpLobby);
                true
            }
        }
        (LocalNetDebugMode::Pvp, LocalNetDebugRole::Client) => {
            pvp_config.mode = PvpNetMode::Client;
            pvp_config.host_ip = config.host_ip.clone();
            if let Err(err) = start_pvp_client_socket(&mut pvp_net) {
                warn!("Local pvp debug client startup failed: {err:?}");
                false
            } else if let Ok(addr) = format!("{}:{}", config.host_ip, PVP_PORT).parse() {
                pvp_net.peer = Some(addr);
                next_state.set(AppState::PvpLobby);
                true
            } else {
                warn!("Invalid local pvp debug host address: {}", config.host_ip);
                false
            }
        }
    };

    if boot_ok {
        runtime.bootstrapped = true;
        info!("Local net debug bootstrapped: {:?} {:?}", mode, role);
    }
}

#[cfg(debug_assertions)]
fn env_flag(name: &str) -> bool {
    env::var(name)
        .ok()
        .map(|value| {
            let trimmed = value.trim();
            !trimmed.is_empty() && trimmed != "0" && !trimmed.eq_ignore_ascii_case("false")
        })
        .unwrap_or(false)
}
