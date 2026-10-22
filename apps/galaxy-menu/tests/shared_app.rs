mod common;

use bevy::{input::ButtonState, prelude::*};
use common::{launch_app, mouse_button, mouse_motion};
use galaxy_menu::{BackgroundStarfield, GalaxyCompanion, GalaxyDecorative, GalaxyGas, MenuAction};

#[test]
fn galaxy_backdrop_presents_only_stellar_and_gas_geometry() {
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);

    let world = app.world_mut();
    assert!(
        world
            .query_filtered::<&Mesh3d, (
                Without<GalaxyDecorative>,
                Without<GalaxyGas>,
                Without<GalaxyCompanion>,
                Without<BackgroundStarfield>,
            )>()
            .iter(world)
            .next()
            .is_none(),
        "the backdrop must not present foreground planets or selection highlights"
    );
}

#[test]
fn shared_application_launches_the_menu_and_interactive_galaxy_together() {
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);

    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<&Mesh3d, With<GalaxyDecorative>>()
            .iter(world)
            .count(),
        1
    );
    assert!(
        world
            .query::<&MenuAction>()
            .iter(world)
            .any(|action| *action == MenuAction::NewGame)
    );
    let camera_before = *world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .unwrap();
    mouse_button(&mut app, ButtonState::Pressed);
    mouse_motion(&mut app, Vec2::new(45.0, 12.0));
    app.update();
    let world = app.world_mut();
    let camera_after = world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .unwrap();
    assert!(
        camera_before.translation.distance(camera_after.translation) > 0.1,
        "the assembled application's galaxy remains interactive"
    );
}
