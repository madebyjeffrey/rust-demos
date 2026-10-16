mod frame_baseline;
mod galaxy_companions;
mod galaxy_gas;
mod galaxy_map;
mod menu;
mod starfield;

pub use galaxy_companions::{GalaxyCompanion, GalaxyCompanionKind};
pub use galaxy_gas::GalaxyGas;
pub use galaxy_map::{BackgroundStarfield, GalaxyDecorative, GalaxyMapPlugin, GalaxyRoot};
pub use menu::{MenuAction, MenuEmphasis, MenuPlugin};

/// Shared scene and menu composition used by desktop and the native iOS executable.
pub struct GalaxyMenuPlugin;

impl bevy::prelude::Plugin for GalaxyMenuPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((GalaxyMapPlugin, MenuPlugin));
        frame_baseline::register(app);
    }
}
