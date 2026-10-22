mod common;

use bevy::{
    input::{ButtonState, mouse::MouseScrollUnit},
    prelude::*,
    render::mesh::VertexAttributeValues,
};
use common::{launch_app, mouse_button, mouse_motion, mouse_scroll, set_viewport};
use galaxy_menu::{BackgroundStarfield, GalaxyCompanion, GalaxyDecorative};

fn launch_scene() -> App {
    launch_app(UVec2::new(1280, 720), 1.0)
}

fn navigate(app: &mut App, motion: Vec2, scroll: f32) {
    mouse_button(app, ButtonState::Pressed);
    mouse_motion(app, motion);
    mouse_scroll(app, MouseScrollUnit::Line, scroll);
    app.update();
}

fn camera_view(app: &mut App) -> (Camera, GlobalTransform) {
    let world = app.world_mut();
    let (camera, transform) = world
        .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
        .single(world)
        .unwrap();
    (camera.clone(), *transform)
}

fn mesh_world_positions<M: Component>(app: &mut App) -> Vec<Vec3> {
    let world = app.world_mut();
    let (mesh, transform) = world
        .query_filtered::<(&Mesh3d, &GlobalTransform), With<M>>()
        .single(world)
        .unwrap();
    let mesh = world.resource::<Assets<Mesh>>().get(&mesh.0).unwrap();
    let VertexAttributeValues::Float32x3(positions) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
    else {
        panic!("expected three-dimensional mesh positions");
    };
    positions
        .iter()
        .map(|position| transform.transform_point(Vec3::from_array(*position)))
        .collect()
}

#[test]
fn background_stars_remain_visible_through_a_full_orbit() {
    let mut app = launch_scene();
    set_viewport(&mut app, UVec2::new(1280, 720), 1.0);
    let stars = mesh_world_positions::<BackgroundStarfield>(&mut app);

    for pitch in [-100_000.0, 100_000.0] {
        for _ in 0..8 {
            navigate(
                &mut app,
                Vec2::new(-std::f32::consts::FRAC_PI_4 / 0.005, pitch),
                -1000.0,
            );
            let (camera, transform) = camera_view(&mut app);
            let visible = stars
                .iter()
                .filter(|position| {
                    camera
                        .world_to_viewport(&transform, **position)
                        .is_ok_and(|point| {
                            point.x >= 0.0
                                && point.x <= 1280.0
                                && point.y >= 0.0
                                && point.y <= 720.0
                        })
                })
                .count();
            assert!(
                visible > 1000,
                "the background star field disappeared: {visible} visible vertices"
            );
        }
    }
}

#[test]
fn closest_zoom_keeps_the_galaxy_backdrop_and_companions_inside_the_viewport() {
    let mut app = launch_scene();
    // Reuse the app so the portrait viewport also exercises a resize while zoomed in.
    for size in [UVec2::new(1280, 720), UVec2::new(720, 900)] {
        set_viewport(&mut app, size, 1.0);
        for pitch in [100_000.0, -100_000.0] {
            for _ in 0..8 {
                navigate(
                    &mut app,
                    Vec2::new(-std::f32::consts::FRAC_PI_4 / 0.005, pitch),
                    1000.0,
                );
                let (camera, transform) = camera_view(&mut app);
                let mut positions = mesh_world_positions::<GalaxyDecorative>(&mut app);
                let world = app.world_mut();
                let companions = world
                    .query_filtered::<(&Mesh3d, &GlobalTransform), With<GalaxyCompanion>>()
                    .iter(world)
                    .map(|(mesh, transform)| (mesh.0.clone(), *transform))
                    .collect::<Vec<_>>();
                for (mesh, companion_transform) in companions {
                    let mesh = world.resource::<Assets<Mesh>>().get(&mesh).unwrap();
                    let VertexAttributeValues::Float32x3(points) =
                        mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
                    else {
                        panic!("expected companion star positions");
                    };
                    positions.extend(points.iter().map(|point| {
                        companion_transform.transform_point(Vec3::from_array(*point))
                    }));
                }
                for position in positions {
                    let point = camera.world_to_viewport(&transform, position).unwrap();
                    assert!(
                        point.x >= 0.0
                            && point.y >= 0.0
                            && point.x <= size.x as f32
                            && point.y <= size.y as f32,
                        "galaxy position {position:?} projects outside {size:?}: {point:?}"
                    );
                }
            }
        }
    }
}
