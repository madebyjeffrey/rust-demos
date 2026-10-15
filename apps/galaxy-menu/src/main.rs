use bevy::prelude::*;
use galaxy_menu::MenuPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MenuPlugin)
        .run();
}
