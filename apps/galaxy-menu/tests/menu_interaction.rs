use std::time::Duration;

use bevy::{
    asset::AssetPlugin,
    camera::{CameraPlugin, RenderTargetInfo},
    image::ImagePlugin,
    input::{
        ButtonState, InputPlugin,
        mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
    },
    mesh::MeshPlugin,
    picking::DefaultPickingPlugins,
    prelude::*,
    text::TextPlugin,
    time::TimeUpdateStrategy,
    ui::{UiGlobalTransform, UiPlugin},
    window::{PrimaryWindow, WindowPlugin, WindowResolution},
};
use galaxy_menu::{
    GalaxyMapPlugin, MenuAction, MenuPlugin, PlanetCardState, PlanetId, PlanetInteractionPlugin,
    PlanetInteractionState, PlanetMarker,
};

fn launch_menu_scene(scale: f32) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        // Initialize the scene's headless material stores before adding UI asset support.
        .add_plugins((GalaxyMapPlugin, PlanetInteractionPlugin))
        .add_plugins((
            AssetPlugin::default(),
            InputPlugin,
            ImagePlugin::default(),
            MeshPlugin,
            CameraPlugin,
            TextPlugin::default(),
            UiPlugin,
            WindowPlugin {
                primary_window: None,
                ..default()
            },
            DefaultPickingPlugins,
            MenuPlugin,
        ))
        .init_asset::<TextureAtlasLayout>();
    let size = UVec2::new((1280.0 * scale) as u32, (720.0 * scale) as u32);
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: WindowResolution::new(size.x, size.y).with_scale_factor_override(scale),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    app.update();
    let world = app.world_mut();
    let (mut camera, mut projection) = world
        .query_filtered::<(&mut Camera, &mut Projection), With<Camera3d>>()
        .single_mut(world)
        .unwrap();
    // Supply renderer viewport information; text measurement and UI layout run normally.
    projection.update(size.x as f32, size.y as f32);
    camera.computed.clip_from_view = projection.get_clip_from_view();
    camera.computed.target_info = Some(RenderTargetInfo {
        physical_size: size,
        scale_factor: scale,
    });
    app.update();
    app.update();
    (app, window)
}

fn mouse_button(app: &mut App, window: Entity, state: ButtonState) {
    app.world_mut().write_message(MouseButtonInput {
        button: MouseButton::Left,
        state,
        window,
    });
}

fn point_cursor(app: &mut App, window: Entity, position: Vec2, click: bool) {
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(position));
    mouse_button(app, window, ButtonState::Released);
    if click {
        mouse_button(app, window, ButtonState::Pressed);
    }
    app.update();
}

#[test]
fn menu_whitespace_allows_planet_hover_and_selection_at_each_display_scale() {
    for scale in [1.0, 2.0] {
        let (mut app, window) = launch_menu_scene(scale);
        mouse_button(&mut app, window, ButtonState::Pressed);
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(-std::f32::consts::PI / 0.005, 0.0),
        });
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: 1000.0,
            window,
            phase: TouchPhase::Moved,
        });
        app.update();

        let world = app.world_mut();
        let position = world
            .query::<(&PlanetMarker, &GlobalTransform)>()
            .iter(world)
            .find(|(planet, _)| planet.id == PlanetId::Halcyon)
            .unwrap()
            .1
            .translation();
        let (camera, transform) = world
            .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
            .single(world)
            .unwrap();
        let cursor = camera.world_to_viewport(transform, position).unwrap();
        // The reproduced destination sits in the transparent space beside Load Game.
        assert!((400.0..434.0).contains(&cursor.x));
        assert!((209.0..248.0).contains(&cursor.y));

        point_cursor(&mut app, window, cursor, false);
        assert_eq!(
            app.world()
                .resource::<PlanetInteractionState>()
                .hovered_planet,
            Some(PlanetId::Halcyon)
        );
        point_cursor(&mut app, window, cursor, true);
        assert_eq!(
            app.world()
                .resource::<PlanetInteractionState>()
                .selected_planet,
            Some(PlanetId::Halcyon)
        );
        assert_eq!(
            app.world().resource::<PlanetCardState>().planet_id,
            Some(PlanetId::Halcyon)
        );
    }
}

#[test]
fn menu_actions_block_planets_behind_their_text_at_each_display_scale() {
    for scale in [1.0, 2.0] {
        let (mut app, window) = launch_menu_scene(scale);
        let world = app.world_mut();
        let action_centers = world
            .query::<(&MenuAction, &ComputedNode, &UiGlobalTransform)>()
            .iter(world)
            .map(|(_, node, transform)| {
                assert!(node.size.x > 0.0 && node.size.y > 0.0);
                transform.translation / scale
            })
            .collect::<Vec<_>>();
        assert_eq!(action_centers.len(), 4);

        for cursor in action_centers {
            let world = app.world_mut();
            let (camera, transform) = world
                .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
                .single(world)
                .unwrap();
            let ray = camera.viewport_to_world(transform, cursor).unwrap();
            let position = ray.origin + ray.direction * 40.0;
            let (planet, parent) = world
                .query::<(Entity, &PlanetMarker, &ChildOf)>()
                .iter(world)
                .find(|(_, marker, _)| marker.id == PlanetId::Vesper)
                .map(|(entity, _, parent)| (entity, parent.parent()))
                .unwrap();
            let local_position = world
                .get::<GlobalTransform>(parent)
                .unwrap()
                .affine()
                .inverse()
                .transform_point3(position);
            world.get_mut::<Transform>(planet).unwrap().translation = local_position;
            app.update();

            point_cursor(&mut app, window, cursor, false);
            assert_eq!(
                app.world()
                    .resource::<PlanetInteractionState>()
                    .hovered_planet,
                None
            );
            point_cursor(&mut app, window, cursor, true);
            assert_eq!(
                app.world()
                    .resource::<PlanetInteractionState>()
                    .selected_planet,
                None
            );
        }
    }
}
