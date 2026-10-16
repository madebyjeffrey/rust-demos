#![allow(dead_code)]

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
    ui::UiPlugin,
    window::{PrimaryWindow, WindowPlugin, WindowResolution},
};
use galaxy_menu::GalaxyMenuPlugin;

/// Assemble the shared application with real input, text measurement, and UI layout,
/// supplying only the GPU's viewport metadata in headless tests.
pub fn launch_app(size: UVec2, scale: f32) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        // Register headless scene material stores before adding UI asset support.
        .add_plugins(GalaxyMenuPlugin)
        .add_plugins((
            AssetPlugin::default(),
            InputPlugin,
            ImagePlugin::default(),
            MeshPlugin,
            CameraPlugin,
            TextPlugin,
            UiPlugin,
            WindowPlugin {
                primary_window: None,
                ..default()
            },
            DefaultPickingPlugins,
        ))
        .init_asset::<TextureAtlasLayout>();
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(size.x, size.y).with_scale_factor_override(scale),
            ..default()
        },
        PrimaryWindow,
    ));
    app.update();
    set_viewport(&mut app, size, scale);
    app.update();
    app.update();
    app
}

pub fn primary_window(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .unwrap()
}

pub fn set_viewport(app: &mut App, size: UVec2, scale: f32) {
    let world = app.world_mut();
    world
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(world)
        .unwrap()
        .resolution = WindowResolution::new(size.x, size.y).with_scale_factor_override(scale);
    let (mut camera, mut projection) = world
        .query_filtered::<(&mut Camera, &mut Projection), With<Camera3d>>()
        .single_mut(world)
        .unwrap();
    projection.update(size.x as f32, size.y as f32);
    camera.computed.clip_from_view = projection.get_clip_from_view();
    camera.computed.target_info = Some(RenderTargetInfo {
        physical_size: size,
        scale_factor: scale,
    });
}

pub fn mouse_button(app: &mut App, state: ButtonState) {
    let window = primary_window(app);
    app.world_mut().write_message(MouseButtonInput {
        button: MouseButton::Left,
        state,
        window,
    });
}

pub fn mouse_motion(app: &mut App, delta: Vec2) {
    app.world_mut().write_message(MouseMotion { delta });
}

pub fn mouse_scroll(app: &mut App, unit: MouseScrollUnit, y: f32) {
    let window = primary_window(app);
    app.world_mut().write_message(MouseWheel {
        unit,
        x: 0.0,
        y,
        window,
        phase: TouchPhase::Moved,
    });
}

pub fn point_cursor(app: &mut App, position: Vec2, click: bool) {
    let window = primary_window(app);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(position));
    mouse_button(app, ButtonState::Released);
    if click {
        mouse_button(app, ButtonState::Pressed);
    }
    app.update();
}
