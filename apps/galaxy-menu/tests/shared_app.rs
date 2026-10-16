use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};
use galaxy_menu::{GalaxyDecorative, GalaxyMenuPlugin, MenuAction};

#[test]
fn shared_application_launches_the_menu_and_interactive_galaxy_together() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .add_plugins(GalaxyMenuPlugin);
    app.update();

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
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    world.resource_mut::<AccumulatedMouseMotion>().delta = Vec2::new(45.0, 12.0);
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
