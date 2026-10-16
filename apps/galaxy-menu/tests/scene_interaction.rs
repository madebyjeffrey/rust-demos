use bevy::{
    asset::Assets,
    camera::RenderTargetInfo,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    math::Affine2,
    prelude::*,
    render::mesh::VertexAttributeValues,
    ui::UiGlobalTransform,
    window::{PrimaryWindow, WindowResolution},
};
use galaxy_menu::{
    BackgroundStarfield, GalaxyCompanion, GalaxyDecorative, GalaxyMapPlugin, PlanetCardName,
    PlanetCardState, PlanetId, PlanetInteractionPlugin, PlanetInteractionState, PlanetMarker,
};

fn launch_scene() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<StandardMaterial>::default())
        .add_plugins((GalaxyMapPlugin, PlanetInteractionPlugin));
    app.update();
    app
}

// Supply the viewport information normally computed by the renderer, without a GPU or OS window.
fn set_viewport(app: &mut App, size: UVec2, scale_factor: f32) {
    let world = app.world_mut();
    let (mut camera, mut projection) = world
        .query_filtered::<(&mut Camera, &mut Projection), With<Camera3d>>()
        .single_mut(world)
        .unwrap();
    projection.update(size.x as f32, size.y as f32);
    camera.computed.clip_from_view = projection.get_clip_from_view();
    camera.computed.target_info = Some(RenderTargetInfo {
        physical_size: size,
        scale_factor,
    });
}

fn navigate(app: &mut App, motion: Vec2, scroll: f32) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut()
        .resource_mut::<AccumulatedMouseMotion>()
        .delta = motion;
    app.world_mut()
        .resource_mut::<AccumulatedMouseScroll>()
        .delta = Vec2::new(0.0, scroll);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .reset_all();
    app.world_mut()
        .resource_mut::<AccumulatedMouseMotion>()
        .delta = Vec2::ZERO;
    app.world_mut()
        .resource_mut::<AccumulatedMouseScroll>()
        .delta = Vec2::ZERO;
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
fn closest_zoom_keeps_the_galaxy_and_destinations_inside_the_viewport() {
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
                positions.extend(
                    world
                        .query_filtered::<&GlobalTransform, With<PlanetMarker>>()
                        .iter(world)
                        .map(GlobalTransform::translation),
                );
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

fn planet_screen_position(app: &mut App, id: PlanetId) -> Vec2 {
    let (camera, camera_transform) = camera_view(app);
    let world = app.world_mut();
    let position = world
        .query::<(&PlanetMarker, &GlobalTransform)>()
        .iter(world)
        .find(|(planet, _)| planet.id == id)
        .unwrap()
        .1
        .translation();
    camera
        .world_to_viewport(&camera_transform, position)
        .unwrap()
}

fn point_window_cursor(app: &mut App, position: Vec2, click: bool) {
    let world = app.world_mut();
    world
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(world)
        .unwrap()
        .set_cursor_position(Some(position));
    let mut buttons = world.resource_mut::<ButtonInput<MouseButton>>();
    buttons.reset_all();
    if click {
        buttons.press(MouseButton::Left);
    }
    app.update();
}

#[test]
fn card_blocks_planet_hover_and_clicks_at_each_display_scale() {
    for scale in [1.0, 2.0] {
        let mut app = launch_scene();
        let size = UVec2::new((1280.0 * scale) as u32, (720.0 * scale) as u32);
        set_viewport(&mut app, size, scale);
        app.world_mut().spawn((
            Window {
                resolution: WindowResolution::new(size.x, size.y).with_scale_factor_override(scale),
                ..default()
            },
            PrimaryWindow,
        ));

        let vesper = planet_screen_position(&mut app, PlanetId::Vesper);
        point_window_cursor(&mut app, vesper, true);
        assert_eq!(
            app.world()
                .resource::<PlanetInteractionState>()
                .selected_planet,
            Some(PlanetId::Vesper)
        );

        let halcyon = planet_screen_position(&mut app, PlanetId::Halcyon);
        let world = app.world_mut();
        let card = world
            .query_filtered::<&ChildOf, With<PlanetCardName>>()
            .single(world)
            .unwrap()
            .parent();
        // Supply rendered card geometry, just as the UI layout pass does. Position it over
        // the destination so hit blocking is independent of the camera-framing policy.
        world.entity_mut(card).insert((
            ComputedNode {
                size: Vec2::new(286.0, 240.0) * scale,
                ..default()
            },
            UiGlobalTransform::from(Affine2::from_translation(halcyon * scale)),
            InheritedVisibility::VISIBLE,
        ));

        point_window_cursor(&mut app, halcyon, false);
        assert_eq!(
            app.world()
                .resource::<PlanetInteractionState>()
                .hovered_planet,
            None
        );
        point_window_cursor(&mut app, halcyon, true);
        assert_eq!(
            app.world()
                .resource::<PlanetInteractionState>()
                .selected_planet,
            Some(PlanetId::Vesper)
        );
        assert_eq!(
            app.world().resource::<PlanetCardState>().planet_id,
            Some(PlanetId::Vesper)
        );

        app.world_mut()
            .entity_mut(card)
            .insert(InheritedVisibility::HIDDEN);
        point_window_cursor(&mut app, halcyon, true);
        assert_eq!(
            app.world()
                .resource::<PlanetInteractionState>()
                .selected_planet,
            Some(PlanetId::Halcyon)
        );
    }
}
