mod common;

use bevy::{camera::RenderTarget, prelude::*, ui::IsDefaultUiCamera};
use common::{launch_app, set_viewport};
use galaxy_menu::{GalaxyRenderSettings, MenuAction};

fn new_game_size(app: &mut App) -> Vec2 {
    let world = app.world_mut();
    world
        .query::<(&MenuAction, &ComputedNode)>()
        .iter(world)
        .find(|(action, _)| **action == MenuAction::NewGame)
        .unwrap()
        .1
        .size()
}

#[test]
fn reducing_scene_resolution_preserves_native_ui_size_and_target() {
    let size = UVec2::new(1200, 2400);
    let mut native = launch_app(size, 3.0);
    let native_button = new_game_size(&mut native);
    let mut reduced = launch_app(size, 3.0);
    reduced.insert_resource(GalaxyRenderSettings { scene_scale: 0.6 });
    reduced.update();
    set_viewport(&mut reduced, size, 3.0);
    reduced.update();
    reduced.update();

    let world = reduced.world_mut();
    let target = world
        .query_filtered::<&RenderTarget, With<Camera3d>>()
        .single(world)
        .unwrap()
        .as_image()
        .expect("reduced scene renders to an image")
        .clone();
    let image = world.resource::<Assets<Image>>().get(&target).unwrap();
    assert_eq!(image.size(), UVec2::new(720, 1440));
    let ui = world
        .query_filtered::<&Camera, With<IsDefaultUiCamera>>()
        .single(world)
        .unwrap();
    assert_eq!(ui.physical_target_size(), Some(size));
    assert_eq!(
        new_game_size(&mut reduced),
        native_button,
        "scene quality must not shrink the labels or touch targets"
    );
}

fn scene_image(app: &mut App) -> Handle<Image> {
    let world = app.world_mut();
    world
        .query_filtered::<&RenderTarget, With<Camera3d>>()
        .single(world)
        .unwrap()
        .as_image()
        .unwrap()
        .clone()
}

fn camera(app: &mut App) -> Transform {
    let world = app.world_mut();
    *world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .unwrap()
}

fn resize(app: &mut App, size: UVec2, scale: f32) {
    set_viewport(app, size, scale);
    app.update();
    // Headless tests supply the GPU metadata after the app resizes its target.
    set_viewport(app, size, scale);
    app.update();
    app.update();
}

#[test]
fn scene_target_tracks_rotation_and_window_resize_without_repeated_image_updates() {
    let mut app = launch_app(UVec2::new(1200, 2400), 3.0);
    app.insert_resource(GalaxyRenderSettings { scene_scale: 0.6 });
    resize(&mut app, UVec2::new(1200, 2400), 3.0);
    let image = scene_image(&mut app);
    let orientation = camera(&mut app).rotation;
    for (native, expected, scale) in [
        (UVec2::new(2400, 1200), UVec2::new(1440, 720), 3.0),
        (UVec2::new(960, 1800), UVec2::new(576, 1080), 2.0),
    ] {
        resize(&mut app, native, scale);
        assert_eq!(
            scene_image(&mut app),
            image,
            "resizing retains the target handle"
        );
        assert_eq!(
            app.world()
                .resource::<Assets<Image>>()
                .get(&image)
                .unwrap()
                .size(),
            expected
        );
        assert_eq!(camera(&mut app).rotation, orientation);
        let world = app.world_mut();
        let projection = world
            .query_filtered::<&Projection, With<Camera3d>>()
            .single(world)
            .unwrap();
        let Projection::Perspective(projection) = projection else {
            panic!("perspective scene")
        };
        assert!((projection.aspect_ratio - native.x as f32 / native.y as f32).abs() < 0.001);
        let ui = world
            .query_filtered::<&Camera, With<IsDefaultUiCamera>>()
            .single(world)
            .unwrap();
        assert_eq!(ui.physical_target_size(), Some(native));
    }
    let mut changes = app
        .world()
        .resource::<Messages<AssetEvent<Image>>>()
        .get_cursor_current();
    app.update();
    app.update();
    assert!(
        !changes
            .read(app.world().resource::<Messages<AssetEvent<Image>>>())
            .any(|event| matches!(event, AssetEvent::Modified { id } if *id == image.id())),
        "an unchanged window must not reallocate the scene texture"
    );
}

#[derive(Resource, Default)]
struct Activations(Vec<MenuAction>);

#[test]
fn scaled_backdrop_keeps_native_menu_touch_ownership_and_background_orbit() {
    use bevy::{input::touch::TouchPhase, ui::UiGlobalTransform, ui_widgets::Activate};
    use common::touch;

    let mut app = launch_app(UVec2::new(1200, 2400), 3.0);
    app.insert_resource(GalaxyRenderSettings { scene_scale: 0.6 });
    app.init_resource::<Activations>();
    app.add_observer(
        |activation: On<Activate>, actions: Query<&MenuAction>, mut log: ResMut<Activations>| {
            if let Ok(action) = actions.get(activation.entity) {
                log.0.push(*action);
            }
        },
    );
    resize(&mut app, UVec2::new(1200, 2400), 3.0);
    let world = app.world_mut();
    let center = world
        .query::<(&MenuAction, &UiGlobalTransform)>()
        .iter(world)
        .find(|(action, _)| **action == MenuAction::NewGame)
        .unwrap()
        .1
        .translation
        / 3.0;
    let before = camera(&mut app);
    touch(&mut app, 1, TouchPhase::Started, center);
    app.update();
    touch(&mut app, 1, TouchPhase::Ended, center);
    app.update();
    assert_eq!(
        app.world().resource::<Activations>().0,
        [MenuAction::NewGame]
    );
    assert_eq!(
        camera(&mut app),
        before,
        "native menu tap must not move the scaled backdrop"
    );
    touch(&mut app, 2, TouchPhase::Started, Vec2::new(380.0, 700.0));
    app.update();
    touch(&mut app, 2, TouchPhase::Moved, Vec2::new(320.0, 660.0));
    app.update();
    assert!(
        camera(&mut app).rotation.angle_between(before.rotation) > 0.1,
        "native whitespace coordinates must continue orbiting the scene"
    );
}
