#![allow(dead_code)]

use std::time::Duration;

use bevy::{
    asset::AssetPlugin,
    camera::{CameraPlugin, RenderTarget, RenderTargetInfo},
    image::ImagePlugin,
    input::{
        ButtonState, InputPlugin,
        keyboard::{Key, KeyboardInput},
        mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
        touch::{TouchInput, TouchPhase},
    },
    input_focus::{InputFocusPlugin, pointer_focus::PointerFocusPlugin},
    mesh::MeshPlugin,
    picking::DefaultPickingPlugins,
    prelude::*,
    text::TextPlugin,
    time::TimeUpdateStrategy,
    ui::UiPlugin,
    ui_widgets::UiWidgetsPlugins,
    window::{
        CursorMoved, PrimaryWindow, WindowEvent, WindowFocused, WindowPlugin, WindowResolution,
    },
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
            InputFocusPlugin,
            UiWidgetsPlugins.build().disable::<PointerFocusPlugin>(),
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
    let mut targets = world.query::<(Entity, &RenderTarget)>();
    let targets = targets
        .iter(world)
        .map(|(entity, target)| {
            let (physical_size, scale_factor) = target
                .as_image()
                .and_then(|handle| world.resource::<Assets<Image>>().get(handle))
                .map_or((size, scale), |image| (image.size(), 1.0));
            (entity, physical_size, scale_factor)
        })
        .collect::<Vec<_>>();
    let mut cameras = world.query::<(&mut Camera, &mut Projection)>();
    for (entity, physical_size, scale_factor) in targets {
        let (mut camera, mut projection) = cameras.get_mut(world, entity).unwrap();
        projection.update(physical_size.x as f32, physical_size.y as f32);
        camera.computed.clip_from_view = projection.get_clip_from_view();
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size,
            scale_factor,
        });
    }
}

pub fn mouse_button(app: &mut App, state: ButtonState) {
    let window = primary_window(app);
    let input = MouseButtonInput {
        button: MouseButton::Left,
        state,
        window,
    };
    app.world_mut().write_message(input);
    app.world_mut()
        .write_message(WindowEvent::MouseButtonInput(input));
}

pub fn mouse_motion(app: &mut App, delta: Vec2) {
    app.world_mut().write_message(MouseMotion { delta });
}

pub fn key(app: &mut App, key_code: KeyCode, state: ButtonState, repeat: bool) {
    let window = primary_window(app);
    let input = KeyboardInput {
        key_code,
        logical_key: match key_code {
            KeyCode::Tab => Key::Tab,
            KeyCode::Enter => Key::Enter,
            KeyCode::Space => Key::Space,
            _ => Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
        },
        state,
        text: None,
        repeat,
        window,
    };
    app.world_mut().write_message(input.clone());
    app.world_mut()
        .write_message(WindowEvent::KeyboardInput(input));
}

pub fn touch(app: &mut App, id: u64, phase: TouchPhase, position: Vec2) {
    let window = primary_window(app);
    let input = TouchInput {
        window,
        id,
        phase,
        position,
        force: None,
    };
    app.world_mut().write_message(input);
    app.world_mut()
        .write_message(WindowEvent::TouchInput(input));
}

pub fn window_focus(app: &mut App, focused: bool) {
    let window = primary_window(app);
    app.world_mut().get_mut::<Window>(window).unwrap().focused = focused;
    let input = WindowFocused { window, focused };
    app.world_mut().write_message(input.clone());
    app.world_mut()
        .write_message(WindowEvent::WindowFocused(input));
}

pub fn mouse_scroll(app: &mut App, unit: MouseScrollUnit, y: f32) {
    mouse_scroll_with_phase(app, unit, y, TouchPhase::Moved);
}

pub fn mouse_scroll_with_phase(app: &mut App, unit: MouseScrollUnit, y: f32, phase: TouchPhase) {
    let window = primary_window(app);
    let input = MouseWheel {
        unit,
        x: 0.0,
        y,
        window,
        phase,
    };
    app.world_mut().write_message(input);
    app.world_mut()
        .write_message(WindowEvent::MouseWheel(input));
}

pub fn point_cursor(app: &mut App, position: Vec2, click: bool) {
    let window = primary_window(app);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(position));
    app.world_mut()
        .write_message(WindowEvent::CursorMoved(CursorMoved {
            window,
            position,
            delta: None,
        }));
    mouse_button(app, ButtonState::Released);
    if click {
        mouse_button(app, ButtonState::Pressed);
    }
    app.update();
}
