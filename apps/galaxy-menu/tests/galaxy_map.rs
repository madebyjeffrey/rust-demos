use bevy::{
    asset::{Assets, RenderAssetUsages},
    picking::Pickable,
    prelude::*,
    render::mesh::VertexAttributeValues,
};
use galaxy_menu::{
    BackgroundStarfield, GalaxyDecorative, GalaxyMapPlugin, GalaxyRoot, MenuPlugin, PlanetMarker,
};

fn launch_map() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
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
