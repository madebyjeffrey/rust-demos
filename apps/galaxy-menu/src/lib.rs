mod frame_baseline;
mod galaxy_companions;
mod galaxy_gas;
mod galaxy_map;
mod menu;
mod planet;
mod planet_interaction;
mod starfield;

pub use galaxy_companions::{GalaxyCompanion, GalaxyCompanionKind};
pub use galaxy_gas::GalaxyGas;
pub use galaxy_map::{BackgroundStarfield, GalaxyDecorative, GalaxyMapPlugin, GalaxyRoot};
pub use menu::{MenuAction, MenuEmphasis, MenuPlugin};
pub use planet::{PlanetId, PlanetMarker, PlanetProfile};
pub use planet_interaction::{
    PlanetCardDetail, PlanetCardName, PlanetCardState, PlanetCursorRay, PlanetHoverLabel,
    PlanetInteractionInput, PlanetInteractionPlugin, PlanetInteractionState,
    PlanetSelectionHighlight,
};

/// Shared scene and menu composition used by desktop and the native iOS executable.
pub struct GalaxyMenuPlugin;

impl bevy::prelude::Plugin for GalaxyMenuPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((GalaxyMapPlugin, PlanetInteractionPlugin, MenuPlugin));
        frame_baseline::register(app);
    }
}
