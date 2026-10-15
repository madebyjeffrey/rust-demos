use bevy::prelude::*;
use galaxy_menu::{GalaxyMapPlugin, MenuPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((GalaxyMapPlugin, MenuPlugin))
        .run();
}
