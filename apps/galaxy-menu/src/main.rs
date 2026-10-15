use bevy::prelude::*;
use galaxy_menu::{GalaxyMapPlugin, MenuPlugin, PlanetInteractionPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((GalaxyMapPlugin, PlanetInteractionPlugin, MenuPlugin))
        .run();
}
