use bevy::{prelude::*, window::WindowResolution};
use lcars_console::LcarsConsolePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LCARS / Science Operations".into(),
                resolution: WindowResolution::new(1280, 800),
                resize_constraints: bevy::window::WindowResizeConstraints {
                    min_width: 960.0,
                    min_height: 600.0,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LcarsConsolePlugin)
        .run();
}
