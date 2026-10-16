mod common;

use bevy::{
    input::{ButtonState, mouse::MouseScrollUnit},
    picking::Pickable,
    prelude::*,
    render::{
        mesh::VertexAttributeValues,
        render_resource::{TextureDimension, TextureFormat},
    },
    time::TimeUpdateStrategy,
};
use common::{launch_app, mouse_button, mouse_motion, mouse_scroll};
use galaxy_menu::{
    BackgroundStarfield, GalaxyCompanion, GalaxyCompanionKind, GalaxyDecorative, GalaxyGas,
    GalaxyRoot,
};
use std::time::Duration;

fn launch_backdrop() -> App {
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
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
fn galaxy_backdrop_launches_with_an_angled_view() {
    let mut app = launch_backdrop();
    let world = app.world_mut();

    let camera_transform = world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .unwrap();
    assert!(camera_transform.translation.y > 0.0);
    assert!(camera_transform.forward().y < -0.2);

    let pickable = world
        .query_filtered::<&Pickable, With<GalaxyDecorative>>()
        .single(world)
        .unwrap();
    assert_eq!(*pickable, Pickable::IGNORE);
}

#[test]
fn galaxy_starfield_has_the_same_composition_between_launches() {
    let first_galaxy = mesh_positions::<GalaxyDecorative>(&mut launch_backdrop());
    let second_galaxy = mesh_positions::<GalaxyDecorative>(&mut launch_backdrop());
    let first_background = mesh_positions::<BackgroundStarfield>(&mut launch_backdrop());
    let second_background = mesh_positions::<BackgroundStarfield>(&mut launch_backdrop());

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
fn dense_stars_and_non_pickable_gas_rotate_as_one_galaxy() {
    let mut app = launch_backdrop();
    let positions = mesh_positions::<GalaxyDecorative>(&mut app);
    let (stars, remainder) = positions.as_chunks::<4>();
    assert!(
        remainder.is_empty(),
        "each star should have a complete quad"
    );
    assert!(
        stars.len() >= 60_000,
        "the rotating body should contain tens of thousands of stars"
    );
    assert!(
        positions
            .iter()
            .all(|point| point.iter().all(|value| value.is_finite()))
    );
    let gas_positions = mesh_positions::<GalaxyGas>(&mut app);
    assert!(gas_positions.iter().any(|point| point[1] > 1.0));
    assert!(gas_positions.iter().any(|point| point[1] < -1.0));
    let world = app.world_mut();
    let root = world
        .query_filtered::<Entity, With<GalaxyRoot>>()
        .single(world)
        .unwrap();
    let (parent, pickable) = world
        .query_filtered::<(&ChildOf, &Pickable), With<GalaxyGas>>()
        .single(world)
        .expect("gas should be one continuous volume rather than separate flat slices");
    assert_eq!(parent.parent(), root);
    assert_eq!(*pickable, Pickable::IGNORE);
    let images = world.resource::<Assets<Image>>();
    let image = images
        .iter()
        .find(|(_, image)| {
            image.texture_descriptor.dimension == TextureDimension::D3
                && image.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb
        })
        .expect("gas should store cloud and dust density in three dimensions")
        .1;
    let size = image.texture_descriptor.size;
    assert!(size.width >= 256 && size.height >= 64 && size.depth_or_array_layers >= 256);
    let pixels = image.data.as_ref().unwrap();
    for z in 0..size.depth_or_array_layers {
        for x in 0..size.width {
            for y in [0, size.height - 1] {
                let index = ((z * size.height + y) * size.width + x) as usize * 4 + 3;
                assert_eq!(
                    pixels[index], 0,
                    "gas must fade out before its volume boundary"
                );
            }
        }
    }
    let dust = images
        .iter()
        .find(|(_, image)| {
            image.texture_descriptor.dimension == TextureDimension::D3
                && image.texture_descriptor.format == TextureFormat::R8Unorm
        })
        .expect("absorbing dust must be independent of the emitting gas")
        .1;
    let pixels = dust.data.as_ref().unwrap();
    let size = dust.texture_descriptor.size;
    let column_sum = |y| {
        (0..size.depth_or_array_layers)
            .flat_map(|z| {
                (0..size.width)
                    .map(move |x| pixels[((z * size.height + y) * size.width + x) as usize] as u64)
            })
            .sum::<u64>()
    };
    assert!(
        column_sum(size.height / 2) > 4 * column_sum(size.height * 3 / 4),
        "dust should concentrate in a thin lane within the luminous bulge"
    );
}

#[test]
fn stellar_companions_are_decorative_and_populate_both_sides_of_the_disk() {
    let mut app = launch_backdrop();
    let world = app.world_mut();
    let root = *world
        .query_filtered::<&Transform, With<GalaxyRoot>>()
        .single(world)
        .unwrap();
    let companions = world
        .query::<(&GalaxyCompanion, &Transform, &Pickable, Option<&ChildOf>)>()
        .iter(world)
        .collect::<Vec<_>>();
    assert_eq!(
        companions
            .iter()
            .filter(|(c, _, _, _)| c.kind == GalaxyCompanionKind::DwarfGalaxy)
            .count(),
        2
    );
    assert_eq!(
        companions
            .iter()
            .filter(|(c, _, _, _)| c.kind == GalaxyCompanionKind::GlobularCluster)
            .count(),
        3
    );
    let heights = companions
        .iter()
        .map(|(_, t, _, _)| (t.translation - root.translation).dot(*root.up()))
        .collect::<Vec<_>>();
    assert!(heights.iter().any(|height| *height > 4.0));
    assert!(heights.iter().any(|height| *height < -4.0));
    for (_, transform, pickable, parent) in companions {
        assert_eq!(*pickable, Pickable::IGNORE);
        assert!(
            parent.is_none(),
            "companions should orbit independently of the disk's spin"
        );
        assert!(transform.translation.distance(root.translation) > 12.0);
    }
}

#[test]
fn companions_follow_different_inclined_orbits_and_pause_for_dragging() {
    let mut app = launch_backdrop();
    let world = app.world_mut();
    let root = *world
        .query_filtered::<&Transform, With<GalaxyRoot>>()
        .single(world)
        .unwrap();
    let initial = world
        .query_filtered::<(Entity, &Transform), With<GalaxyCompanion>>()
        .iter(world)
        .map(|(entity, transform)| (entity, transform.translation))
        .collect::<Vec<_>>();
    for _ in 0..100 {
        app.update();
    }
    let world = app.world_mut();
    let mut normals = Vec::new();
    let mut moved = Vec::new();
    for (entity, before) in initial {
        let after = world.get::<Transform>(entity).unwrap().translation;
        assert!(
            after.distance(before) > 0.4 && after.distance(before) < 3.0,
            "companions should drift slowly along their orbits"
        );
        let normal = (before - root.translation)
            .cross(after - root.translation)
            .normalize();
        assert!(
            normal.dot(*root.up()).abs() < 0.95,
            "companion orbits should be inclined relative to the galaxy disk"
        );
        normals.push(normal);
        moved.push((entity, after));
    }
    assert!(
        normals[0].dot(normals[1]).abs() < 0.95,
        "the companion orbits should use different planes"
    );
    mouse_button(&mut app, ButtonState::Pressed);
    mouse_motion(&mut app, Vec2::new(20.0, 10.0));
    for _ in 0..5 {
        app.update();
    }
    for (entity, position) in moved {
        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().translation,
            position
        );
    }
}

#[test]
fn mouse_drag_orbits_the_camera_around_the_galaxy_backdrop() {
    let mut app = launch_backdrop();

    let before = *app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap();
    mouse_button(&mut app, ButtonState::Pressed);
    mouse_motion(&mut app, Vec2::new(90.0, -35.0));

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
fn scroll_zoom_moves_the_camera_toward_the_galaxy_backdrop() {
    let mut app = launch_backdrop();

    let target = Vec3::new(3.8, 0.0, 0.0);
    let before = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .translation
        .distance(target);
    mouse_scroll(&mut app, MouseScrollUnit::Line, 1.0);

    app.update();

    let after = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .translation
        .distance(target);
    assert!(after < before, "scrolling forward should zoom in");

    mouse_scroll(&mut app, MouseScrollUnit::Line, 1_000.0);
    app.update();
    let closest = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .translation
        .distance(target);
    assert!(closest >= 18.0 && closest < before);

    mouse_scroll(&mut app, MouseScrollUnit::Line, -2_000.0);
    app.update();
    let farthest = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .translation
        .distance(target);
    assert!(farthest > before);
    mouse_button(&mut app, ButtonState::Pressed);
    mouse_scroll(&mut app, MouseScrollUnit::Line, -2_000.0);
    app.update();
    let bounded = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .translation
        .distance(target);
    assert!(
        (bounded - farthest).abs() < 0.001,
        "zoom remains bounded relative to the fitted galaxy"
    );
}

#[test]
fn vertical_drag_completes_full_orbits_without_stopping_or_flipping() {
    let mut app = launch_backdrop();
    mouse_button(&mut app, ButtonState::Pressed);
    let initial = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(app.world())
        .unwrap()
        .rotation;
    let step = std::f32::consts::TAU / 32.0;
    for direction in [-1.0, 1.0] {
        let mut previous = initial;
        let mut visited_above = false;
        let mut visited_below = false;
        for _ in 0..64 {
            mouse_motion(&mut app, Vec2::new(0.0, direction * step / 0.005));
            app.update();
            let transform = app
                .world_mut()
                .query_filtered::<&Transform, With<Camera3d>>()
                .single(app.world())
                .unwrap();
            assert!(transform.translation.is_finite() && transform.rotation.is_finite());
            let movement = previous.angle_between(transform.rotation);
            assert!(
                (movement - step).abs() < 0.002,
                "vertical dragging stopped or flipped: expected {step}, got {movement}"
            );
            visited_above |= transform.translation.y > 1.0;
            visited_below |= transform.translation.y < -1.0;
            previous = transform.rotation;
        }
        assert!(
            visited_above && visited_below,
            "the orbit should reach both sides of the galaxy"
        );
        assert!(
            initial.angle_between(previous) < 0.002,
            "full turns should return to the original orientation"
        );
    }
}

#[test]
fn galaxy_idles_while_background_stars_stay_fixed() {
    let mut app = launch_backdrop();
    let root_before = app
        .world_mut()
        .query_filtered::<&Transform, With<GalaxyRoot>>()
        .single(app.world())
        .unwrap()
        .rotation;
    let background_before = *app
        .world_mut()
        .query_filtered::<&Transform, With<BackgroundStarfield>>()
        .single(app.world())
        .unwrap();

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

#[test]
fn galaxy_rotation_pauses_while_camera_input_is_active() {
    let mut app = launch_backdrop();
    let root_before = *app
        .world_mut()
        .query_filtered::<&Transform, With<GalaxyRoot>>()
        .single(app.world())
        .unwrap();
    mouse_button(&mut app, ButtonState::Pressed);
    mouse_motion(&mut app, Vec2::new(90.0, -35.0));

    app.update();

    let root_after = app
        .world_mut()
        .query_filtered::<&Transform, With<GalaxyRoot>>()
        .single(app.world())
        .unwrap();
    assert_eq!(*root_after, root_before);
}
