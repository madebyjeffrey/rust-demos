use bevy::{
    input_focus::{InputDispatchPlugin, pointer_focus::PointerFocusPlugin},
    prelude::*,
};
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
        // MenuPlugin owns native input order; frame-final focus/picking would reorder it.
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<InputDispatchPlugin>()
                .disable::<PointerFocusPlugin>(),
        )
        .add_plugins(GalaxyMenuPlugin)
        .run();
}
