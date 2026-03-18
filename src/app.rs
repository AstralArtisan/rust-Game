use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::core::{
    assets::AssetsPlugin, audio::AudioPlugin, camera::CameraPlugin, events::EventsPlugin,
    input::InputPlugin,
};
use crate::data::DataPlugin;
use crate::gameplay::GameplayPlugin;
use crate::coop::CoopPlugin;
use crate::pvp::PvpPlugin;
use crate::states::AppState;
use crate::ui::UiPlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>().add_plugins((
            EventsPlugin,
            AssetsPlugin,
            DataPlugin,
            InputPlugin,
            AudioPlugin,
            CameraPlugin,
            GameplayPlugin,
            UiPlugin,
        ));
    }
}
