use bevy::{
    asset::{Assets, RenderAssetUsages},
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    picking::Pickable,
    prelude::*,
    render::mesh::VertexAttributeValues,
    time::TimeUpdateStrategy,
};
use galaxy_menu::{
    BackgroundStarfield, GalaxyDecorative, GalaxyMapPlugin, GalaxyRoot, MenuPlugin, PlanetMarker,
};
use std::time::Duration;

fn launch_map() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )))
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<StandardMaterial>::default())
        .add_plugins((GalaxyMapPlugin, MenuPlugin));
    app.update();
    app
}

fn mesh_positions<M: Component>(app: &mut App) -> Vec<[f32; 3]> {
    let world = app.world_mut();
    let handle = world
        .query_filtered::<&Mesh3d, With<M>>()
        .single(world)
        .unwrap()
        .0
        .clone();
    let mesh = world.resource::<Assets<Mesh>>().get(&handle).unwrap();
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
        VertexAttributeValues::Float32x3(positions) => positions.clone(),
        _ => panic!("galaxy positions should use three dimensional vertices"),
    }
}

#[test]
fn galaxy_map_launches_with_an_angle_and_a_rotation_ready_root() {
    let mut app = launch_map();
    let world = app.world_mut();

    let root = world
        .query_filtered::<(Entity, &Visibility), With<GalaxyRoot>>()
        .single(world)
        .unwrap()
        .0;
    let camera_transform = world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .unwrap();
    assert!(camera_transform.translation.y > 0.0);
    assert!(camera_transform.forward().y < -0.2);

    let planets = world
        .query::<(&PlanetMarker, &ChildOf)>()
        .iter(world)
        .collect::<Vec<_>>();
    assert_eq!(planets.len(), 5);
    assert!(planets.iter().all(|(_, parent)| parent.parent() == root));

    let pickable = world
        .query_filtered::<&Pickable, With<GalaxyDecorative>>()
        .single(world)
        .unwrap();
    assert_eq!(*pickable, Pickable::IGNORE);
}

#[test]
fn galaxy_starfield_has_the_same_composition_between_launches() {
    let first_galaxy = mesh_positions::<GalaxyDecorative>(&mut launch_map());
    let second_galaxy = mesh_positions::<GalaxyDecorative>(&mut launch_map());
    let first_background = mesh_positions::<BackgroundStarfield>(&mut launch_map());
    let second_background = mesh_positions::<BackgroundStarfield>(&mut launch_map());

    assert!(
        first_galaxy.len() > 1_000,
        "the galaxy should read as a broad spiral"
    );
    assert!(
        first_background.len() > 1_000,
        "the background should contain many stars"
    );
    assert_eq!(first_galaxy, second_galaxy);
    assert_eq!(first_background, second_background);
}

#[test]
fn planet_markers_are_few_and_keep_cpu_mesh_data_for_picking() {
    let mut app = launch_map();
    let world = app.world_mut();
    let mut query = world.query::<(&PlanetMarker, &Mesh3d)>();
    let planets = query.iter(world).collect::<Vec<_>>();

    assert!((3..=7).contains(&planets.len()));
    for (_, handle) in planets {
        let mesh = world.resource::<Assets<Mesh>>().get(&handle.0).unwrap();
        assert!(mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD));
    }
}

#[test]
fn mouse_drag_orbits_the_camera_around_the_galaxy_map() {
    let mut app = launch_map();

    let before = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .clone();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut()
        .resource_mut::<AccumulatedMouseMotion>()
        .delta = Vec2::new(90.0, -35.0);

    app.update();

    let after = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap();
    assert!(after.translation.distance(before.translation) > 1.0);
    assert!(after.forward().distance(*before.forward()) > 0.05);
}

#[test]
fn scroll_zoom_moves_the_camera_toward_the_galaxy_map() {
    let mut app = launch_map();

    let target = Vec3::new(3.8, 0.0, 0.0);
    let before = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .translation
        .distance(target);
    app.world_mut()
        .resource_mut::<AccumulatedMouseScroll>()
        .delta = Vec2::new(0.0, 1.0);

    app.update();

    let after = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .translation
        .distance(target);
    assert!(after < before, "scrolling forward should zoom in");

    app.world_mut()
        .resource_mut::<AccumulatedMouseScroll>()
        .delta = Vec2::new(0.0, 1_000.0);
    app.update();
    let closest = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .translation
        .distance(target);
    assert!((closest - 18.0).abs() < 0.001);

    app.world_mut()
        .resource_mut::<AccumulatedMouseScroll>()
        .delta = Vec2::new(0.0, -2_000.0);
    app.update();
    let farthest = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .translation
        .distance(target);
    assert!((farthest - 72.0).abs() < 0.001);
}

#[test]
fn mouse_drag_keeps_camera_pitch_away_from_vertical_views() {
    let mut app = launch_map();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);

    app.world_mut()
        .resource_mut::<AccumulatedMouseMotion>()
        .delta = Vec2::new(0.0, 100_000.0);
    app.update();
    let upper_view = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap();
    assert!(upper_view.forward().y < -0.9 && upper_view.forward().y > -0.99);

    app.world_mut()
        .resource_mut::<AccumulatedMouseMotion>()
        .delta = Vec2::new(0.0, -200_000.0);
    app.update();
    let lower_view = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap();
    assert!(lower_view.forward().y > 0.9 && lower_view.forward().y < 0.99);
}

#[test]
fn galaxy_idles_while_background_stars_stay_fixed() {
    let mut app = launch_map();
    let root_before = app
        .world_mut()
        .query_filtered::<&Transform, With<GalaxyRoot>>()
        .single(app.world())
        .unwrap()
        .rotation;
    let background_before = app
        .world_mut()
        .query_filtered::<&Transform, With<BackgroundStarfield>>()
        .single(app.world())
        .unwrap()
        .clone();

    app.update();

    let root_after = app
        .world_mut()
        .query_filtered::<&Transform, With<GalaxyRoot>>()
        .single(app.world())
        .unwrap()
        .rotation;
    let background_after = app
        .world_mut()
        .query_filtered::<&Transform, With<BackgroundStarfield>>()
        .single(app.world())
        .unwrap();
    let idle_rotation = root_before.angle_between(root_after);
    assert!(idle_rotation > 0.0 && idle_rotation < 0.01);
    assert_eq!(*background_after, background_before);
}
