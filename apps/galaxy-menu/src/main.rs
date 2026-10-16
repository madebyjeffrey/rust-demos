use bevy::prelude::*;
use galaxy_menu::GalaxyMenuPlugin;

#[cfg(target_os = "ios")]
unsafe extern "C" {
    fn galaxy_register_scene();
}

fn main() {
    #[cfg(target_os = "ios")]
    // SAFETY: UIKit scene registration runs once on the executable's main thread.
    unsafe {
        galaxy_register_scene()
    };
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GalaxyMenuPlugin)
        .run();
}
