use bevy::prelude::*;
use galaxy_menu::{GalaxyMapPlugin, MenuPlugin, PlanetInteractionPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Keep scene creation, interaction, and the menu shell in independent plugins.
        .add_plugins((GalaxyMapPlugin, PlanetInteractionPlugin, MenuPlugin))
        .run();
}
