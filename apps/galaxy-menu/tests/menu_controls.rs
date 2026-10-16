mod common;

use bevy::{
    input::ButtonState,
    prelude::*,
    ui::UiGlobalTransform,
    ui_widgets::{Activate, Button},
};
use common::{
    key, launch_app, mouse_button, mouse_motion, mouse_scroll, point_cursor, touch, window_focus,
};
use galaxy_menu::MenuAction;

#[test]
fn menu_actions_are_readable_buttons_on_the_baseline_device_windows() {
    for size in [UVec2::new(402, 874), UVec2::new(744, 1133)] {
        let mut app = launch_app(size, 1.0);
        let world = app.world_mut();
        let buttons = world
            .query::<(&MenuAction, &Button, &ComputedNode, &Text)>()
            .iter(world)
            .map(|(action, _, node, text)| (*action, node.size(), text.0.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            buttons.len(),
            if cfg!(target_os = "ios") { 3 } else { 4 },
            "menu actions must be real buttons, with Quit only on desktop"
        );
        assert_eq!(
            buttons
                .iter()
                .any(|(action, _, _)| *action == MenuAction::Quit),
            !cfg!(target_os = "ios")
        );
        for (_, bounds, label) in buttons {
            assert!(bounds.x >= 200.0 && bounds.y >= 44.0, "{label}: {bounds}");
            assert!(bounds.x < size.x as f32, "{label} must fit the window");
        }
    }
}

#[test]
fn desktop_buttons_show_pointer_feedback_and_activate_placeholder_actions() {
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    app.init_resource::<Activations>();
    app.add_observer(
        |event: On<Activate>, actions: Query<&MenuAction>, mut log: ResMut<Activations>| {
            if let Ok(action) = actions.get(event.entity) {
                log.0.push(*action);
            }
        },
    );
    for action in [
        MenuAction::NewGame,
        MenuAction::LoadGame,
        MenuAction::Settings,
    ] {
        point_cursor(&mut app, Vec2::new(1100.0, 650.0), false);
        let (entity, center) = button(&mut app, action);
        let idle = *app.world().get::<BackgroundColor>(entity).unwrap();
        point_cursor(&mut app, center, false);
        let hovered = *app.world().get::<BackgroundColor>(entity).unwrap();
        assert_ne!(idle, hovered, "{action:?} needs hover feedback");
        mouse_button(&mut app, ButtonState::Pressed);
        app.update();
        let pressed = *app.world().get::<BackgroundColor>(entity).unwrap();
        assert_ne!(hovered, pressed, "{action:?} needs pressed feedback");
        assert!(app.world().resource::<Activations>().0.is_empty());
        mouse_button(&mut app, ButtonState::Released);
        app.update();
        assert_eq!(app.world().resource::<Activations>().0, [action]);
        assert!(
            app.world().resource::<Messages<AppExit>>().is_empty(),
            "placeholder actions keep the menu open"
        );
        app.world_mut().resource_mut::<Activations>().0.clear();
    }
}

#[derive(Resource, Default)]
struct Activations(Vec<MenuAction>);

#[test]
fn keyboard_navigation_and_activation_show_focus_and_press_feedback_without_repeating() {
    use bevy::input_focus::InputFocus;
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    app.init_resource::<Activations>();
    app.add_observer(
        |event: On<Activate>, actions: Query<&MenuAction>, mut log: ResMut<Activations>| {
            if let Ok(action) = actions.get(event.entity) {
                log.0.push(*action);
            }
        },
    );
    let (new_game, _) = button(&mut app, MenuAction::NewGame);
    let resting_border = *app.world().get::<BorderColor>(new_game).unwrap();
    for action in [
        MenuAction::NewGame,
        MenuAction::LoadGame,
        MenuAction::Settings,
        MenuAction::Quit,
        MenuAction::NewGame,
    ] {
        key(&mut app, KeyCode::Tab, ButtonState::Pressed, false);
        app.update();
        let (entity, _) = button(&mut app, action);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(entity));
        assert_ne!(
            *app.world().get::<BorderColor>(entity).unwrap(),
            resting_border,
            "focused action must be visible"
        );
        key(&mut app, KeyCode::Tab, ButtonState::Released, false);
        app.update();
    }
    for key_code in [KeyCode::Enter, KeyCode::Space] {
        let resting = *app.world().get::<BackgroundColor>(new_game).unwrap();
        key(&mut app, key_code, ButtonState::Pressed, false);
        app.update();
        assert_eq!(
            app.world().resource::<Activations>().0,
            [MenuAction::NewGame]
        );
        assert_ne!(
            *app.world().get::<BackgroundColor>(new_game).unwrap(),
            resting,
            "keyboard presses need feedback too"
        );
        key(&mut app, key_code, ButtonState::Pressed, true);
        app.update();
        assert_eq!(
            app.world().resource::<Activations>().0,
            [MenuAction::NewGame]
        );
        key(&mut app, key_code, ButtonState::Released, false);
        app.update();
        assert_eq!(
            *app.world().get::<BackgroundColor>(new_game).unwrap(),
            resting
        );
        app.world_mut().resource_mut::<Activations>().0.clear();
    }
    key(&mut app, KeyCode::ShiftLeft, ButtonState::Pressed, false);
    key(&mut app, KeyCode::Tab, ButtonState::Pressed, false);
    app.update();
    let (quit, _) = button(&mut app, MenuAction::Quit);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(quit));
    key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
    app.update();
    assert!(
        app.world()
            .resource::<Messages<AppExit>>()
            .iter_current_update_messages()
            .any(|exit| *exit == AppExit::Success)
    );
}

fn button(app: &mut App, action: MenuAction) -> (Entity, Vec2) {
    let world = app.world_mut();
    world
        .query::<(Entity, &MenuAction, &UiGlobalTransform)>()
        .iter(world)
        .find(|(_, candidate, _)| **candidate == action)
        .map(|(entity, _, transform)| (entity, transform.translation))
        .unwrap()
}

#[test]
fn mouse_press_on_a_control_owns_its_drag_and_wheel_until_release() {
    use bevy::input::mouse::MouseScrollUnit;
    for scale in [1.0, 2.0] {
        let mut app = launch_app(UVec2::new(1280, 720) * scale as u32, scale);
        let (_, center) = button(&mut app, MenuAction::NewGame);
        point_cursor(&mut app, center / scale, false);
        let before = camera(&mut app);
        mouse_button(&mut app, ButtonState::Pressed);
        mouse_motion(&mut app, Vec2::new(70.0, 20.0));
        mouse_scroll(&mut app, MouseScrollUnit::Line, -1.0);
        app.update();
        assert_eq!(
            camera(&mut app),
            before,
            "pressing a control cannot also move the camera"
        );
        move_cursor(&mut app, Vec2::new(1000.0, 600.0));
        mouse_motion(&mut app, Vec2::new(70.0, 20.0));
        mouse_scroll(&mut app, MouseScrollUnit::Line, -1.0);
        app.update();
        assert_eq!(
            camera(&mut app),
            before,
            "dragging outside keeps UI ownership"
        );
        mouse_button(&mut app, ButtonState::Released);
        app.update();
        mouse_button(&mut app, ButtonState::Pressed);
        mouse_motion(&mut app, Vec2::new(70.0, 20.0));
        app.update();
        assert!(
            before.rotation.angle_between(camera(&mut app).rotation) > 0.1,
            "the next backdrop drag must work"
        );
    }
}

fn camera(app: &mut App) -> Transform {
    let world = app.world_mut();
    *world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .unwrap()
}

fn move_cursor(app: &mut App, position: Vec2) {
    use bevy::window::{CursorMoved, WindowEvent};
    let window = common::primary_window(app);
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
}

#[test]
fn touch_controls_tap_and_keep_ownership_through_drag_and_cancellation() {
    use bevy::input::{mouse::MouseScrollUnit, touch::TouchPhase};
    for scale in [1.0, 2.0] {
        let mut app = launch_app(UVec2::new(1280, 720) * scale as u32, scale);
        app.init_resource::<Activations>();
        app.add_observer(
            |event: On<Activate>, actions: Query<&MenuAction>, mut log: ResMut<Activations>| {
                if let Ok(action) = actions.get(event.entity) {
                    log.0.push(*action);
                }
            },
        );
        let (entity, center) = button(&mut app, MenuAction::NewGame);
        let center = center / scale;
        let idle = *app.world().get::<BackgroundColor>(entity).unwrap();
        let before = camera(&mut app);
        touch(&mut app, 7, TouchPhase::Started, center);
        app.update();
        assert_ne!(
            *app.world().get::<BackgroundColor>(entity).unwrap(),
            idle,
            "touch down needs pressed feedback"
        );
        touch(&mut app, 7, TouchPhase::Moved, Vec2::new(1000.0, 600.0));
        mouse_scroll(&mut app, MouseScrollUnit::Line, 1.0);
        app.update();
        assert_eq!(
            camera(&mut app),
            before,
            "a touch that began on a button keeps UI ownership outside it"
        );
        touch(&mut app, 7, TouchPhase::Canceled, Vec2::new(1000.0, 600.0));
        app.update();
        assert_eq!(*app.world().get::<BackgroundColor>(entity).unwrap(), idle);
        assert!(
            app.world().resource::<Activations>().0.is_empty(),
            "cancelled drags cannot activate"
        );
        touch(&mut app, 8, TouchPhase::Started, center);
        app.update();
        touch(&mut app, 8, TouchPhase::Ended, center);
        app.update();
        assert_eq!(
            app.world().resource::<Activations>().0,
            [MenuAction::NewGame]
        );
        app.update();
        mouse_scroll(&mut app, MouseScrollUnit::Line, 1.0);
        app.update();
        assert_ne!(
            camera(&mut app),
            before,
            "touch release must leave no stale camera ownership"
        );
    }
}

#[test]
fn losing_window_focus_cancels_menu_presses_and_suppresses_a_stale_mouse_drag() {
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    let (entity, center) = button(&mut app, MenuAction::NewGame);
    let idle = *app.world().get::<BackgroundColor>(entity).unwrap();
    point_cursor(&mut app, center, true);
    key(&mut app, KeyCode::Space, ButtonState::Pressed, false);
    app.update();
    assert_ne!(*app.world().get::<BackgroundColor>(entity).unwrap(), idle);
    let before = camera(&mut app);
    window_focus(&mut app, false);
    move_cursor(&mut app, Vec2::new(1000.0, 600.0));
    mouse_motion(&mut app, Vec2::new(70.0, 20.0));
    app.update();
    assert_eq!(
        *app.world().get::<BackgroundColor>(entity).unwrap(),
        idle,
        "focus loss clears pressed feedback"
    );
    assert_eq!(camera(&mut app), before);
    window_focus(&mut app, true);
    mouse_motion(&mut app, Vec2::new(70.0, 20.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        before,
        "a stale held mouse must not become a backdrop drag"
    );
    mouse_button(&mut app, ButtonState::Released);
    key(&mut app, KeyCode::Space, ButtonState::Released, false);
    app.update();
    mouse_button(&mut app, ButtonState::Pressed);
    mouse_motion(&mut app, Vec2::new(70.0, 20.0));
    app.update();
    assert!(before.rotation.angle_between(camera(&mut app).rotation) > 0.1);
}
