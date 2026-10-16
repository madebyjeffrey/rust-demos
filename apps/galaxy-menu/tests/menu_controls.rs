mod common;

use bevy::{
    input::ButtonState,
    prelude::*,
    ui::UiGlobalTransform,
    ui_widgets::{Activate, Button},
};
use common::{
    key, launch_app, mouse_button, mouse_motion, mouse_scroll, mouse_scroll_with_phase,
    point_cursor, touch, window_focus,
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
    let mut app = launch_logged_app(1.0);
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
        key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
        app.update();
        key(&mut app, KeyCode::Enter, ButtonState::Released, false);
        app.update();
        assert_eq!(
            app.world().resource::<Activations>().0,
            [action, action],
            "a mouse-selected control remains operable with the keyboard"
        );
        app.world_mut().resource_mut::<Activations>().0.clear();
    }
}

#[derive(Resource, Default)]
struct Activations(Vec<MenuAction>);

#[test]
fn keyboard_navigation_and_activation_show_focus_and_press_feedback_without_repeating() {
    use bevy::input_focus::InputFocus;
    let mut app = launch_logged_app(1.0);
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
        let mut app = launch_logged_app(scale);
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

#[test]
fn a_mouse_press_keeps_its_starting_owner_when_movement_arrives_in_the_same_frame() {
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    let (_, center) = button(&mut app, MenuAction::NewGame);
    let before = camera(&mut app);
    move_cursor(&mut app, center);
    mouse_button(&mut app, ButtonState::Pressed);
    move_cursor(&mut app, Vec2::new(1000.0, 600.0));
    mouse_motion(&mut app, Vec2::new(70.0, 20.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        before,
        "ownership comes from the press position, even if the cursor has already moved away by the next frame"
    );
}

#[test]
fn wheel_input_over_a_button_does_not_zoom_when_the_cursor_moves_away_in_the_same_frame() {
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    let (_, center) = button(&mut app, MenuAction::NewGame);
    let before = camera(&mut app);
    move_cursor(&mut app, center);
    mouse_scroll(&mut app, bevy::input::mouse::MouseScrollUnit::Line, -1.0);
    move_cursor(&mut app, Vec2::new(1000.0, 600.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        before,
        "wheel ownership comes from its input position"
    );
}

#[test]
fn a_batched_mouse_drag_between_controls_cannot_activate_the_release_target() {
    let mut app = launch_logged_app(1.0);
    let (_, new_game) = button(&mut app, MenuAction::NewGame);
    let (_, load_game) = button(&mut app, MenuAction::LoadGame);
    move_cursor(&mut app, new_game);
    mouse_button(&mut app, ButtonState::Pressed);
    move_cursor(&mut app, load_game);
    app.update();
    mouse_button(&mut app, ButtonState::Released);
    app.update();
    assert!(
        app.world().resource::<Activations>().0.is_empty(),
        "releasing a cross-button drag must not activate either action"
    );
}

#[test]
fn a_batched_touch_drag_between_controls_cannot_activate_the_release_target() {
    use bevy::input::touch::TouchPhase;
    for scale in [1.0, 2.0] {
        let mut app = launch_logged_app(scale);
        let (_, new_game) = button(&mut app, MenuAction::NewGame);
        let (_, load_game) = button(&mut app, MenuAction::LoadGame);
        touch(&mut app, 1, TouchPhase::Started, new_game / scale);
        touch(&mut app, 1, TouchPhase::Moved, load_game / scale);
        app.update();
        touch(&mut app, 1, TouchPhase::Ended, load_game / scale);
        app.update();
        assert!(
            app.world().resource::<Activations>().0.is_empty(),
            "releasing a cross-button touch drag must not activate either action"
        );
    }
}

#[test]
fn a_touch_tap_completed_between_frames_activates_its_original_control_once() {
    use bevy::input::touch::TouchPhase;
    for scale in [1.0, 2.0] {
        let mut app = launch_logged_app(scale);
        let (_, center) = button(&mut app, MenuAction::NewGame);
        touch(&mut app, 1, TouchPhase::Started, center / scale);
        touch(&mut app, 1, TouchPhase::Ended, center / scale);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<Activations>().0,
            [MenuAction::NewGame],
            "short taps must survive input batching without duplicate activation"
        );
    }
}

#[test]
fn cancelling_one_of_two_touches_keeps_the_other_control_press_and_activation() {
    use bevy::input::touch::TouchPhase;
    let mut app = launch_logged_app(1.0);
    let (entity, center) = button(&mut app, MenuAction::NewGame);
    let idle = *app.world().get::<BackgroundColor>(entity).unwrap();
    touch(&mut app, 1, TouchPhase::Started, center);
    app.update();
    touch(&mut app, 2, TouchPhase::Started, center);
    app.update();
    let pressed = *app.world().get::<BackgroundColor>(entity).unwrap();
    assert_ne!(pressed, idle);
    touch(&mut app, 1, TouchPhase::Canceled, center);
    app.update();
    assert_eq!(
        *app.world().get::<BackgroundColor>(entity).unwrap(),
        pressed,
        "the second finger still owns a held press"
    );
    assert!(app.world().resource::<Activations>().0.is_empty());
    touch(&mut app, 2, TouchPhase::Ended, center);
    app.update();
    assert_eq!(*app.world().get::<BackgroundColor>(entity).unwrap(), idle);
    assert_eq!(
        app.world().resource::<Activations>().0,
        [MenuAction::NewGame]
    );
}

fn launch_logged_app(scale: f32) -> App {
    let mut app = launch_app(UVec2::new(1280, 720) * scale as u32, scale);
    app.init_resource::<Activations>();
    app.add_observer(
        |event: On<Activate>, actions: Query<&MenuAction>, mut log: ResMut<Activations>| {
            if let Ok(action) = actions.get(event.entity) {
                log.0.push(*action);
            }
        },
    );
    app
}

#[test]
fn focus_round_trip_cancels_a_batched_mouse_click() {
    let mut app = launch_logged_app(1.0);
    let (_, center) = button(&mut app, MenuAction::Quit);
    move_cursor(&mut app, center);
    mouse_button(&mut app, ButtonState::Pressed);
    window_focus(&mut app, false);
    window_focus(&mut app, true);
    mouse_button(&mut app, ButtonState::Released);
    app.update();
    assert!(
        app.world().resource::<Activations>().0.is_empty(),
        "focus loss should cancel the click; got {:?}",
        app.world().resource::<Activations>().0
    );
    assert!(app.world().resource::<Messages<AppExit>>().is_empty());
}

#[test]
fn focus_round_trip_cancels_a_batched_touch_tap() {
    use bevy::input::touch::TouchPhase;
    let mut app = launch_logged_app(1.0);
    let (_, center) = button(&mut app, MenuAction::NewGame);
    touch(&mut app, 1, TouchPhase::Started, center);
    window_focus(&mut app, false);
    window_focus(&mut app, true);
    touch(&mut app, 1, TouchPhase::Ended, center);
    app.update();
    assert!(
        app.world().resource::<Activations>().0.is_empty(),
        "focus loss should cancel the tap; got {:?}",
        app.world().resource::<Activations>().0
    );
}

#[test]
fn trackpad_scroll_keeps_its_starting_ui_owner() {
    use bevy::input::{mouse::MouseScrollUnit, touch::TouchPhase};
    for scale in [1.0, 2.0] {
        for ending in [TouchPhase::Ended, TouchPhase::Canceled] {
            let mut app = launch_logged_app(scale);
            let (_, center) = button(&mut app, MenuAction::NewGame);
            point_cursor(&mut app, center / scale, false);
            let before = camera(&mut app);
            mouse_scroll_with_phase(
                &mut app,
                MouseScrollUnit::Pixel,
                -100.0,
                TouchPhase::Started,
            );
            app.update();
            assert_eq!(camera(&mut app), before);
            move_cursor(&mut app, Vec2::new(1000.0, 600.0));
            for phase in [TouchPhase::Moved, ending] {
                mouse_scroll_with_phase(&mut app, MouseScrollUnit::Pixel, -100.0, phase);
                app.update();
                assert_eq!(
                    camera(&mut app),
                    before,
                    "UI ownership includes the final scroll event"
                );
            }
            mouse_scroll_with_phase(
                &mut app,
                MouseScrollUnit::Pixel,
                -100.0,
                TouchPhase::Started,
            );
            app.update();
            assert_ne!(
                camera(&mut app),
                before,
                "the next backdrop gesture must zoom"
            );
        }
    }
}

#[test]
fn backdrop_scroll_keeps_its_owner_when_crossing_a_control() {
    use bevy::input::{mouse::MouseScrollUnit, touch::TouchPhase};
    let mut app = launch_logged_app(1.0);
    point_cursor(&mut app, Vec2::new(1000.0, 600.0), false);
    mouse_scroll_with_phase(&mut app, MouseScrollUnit::Pixel, -30.0, TouchPhase::Started);
    app.update();
    let (_, center) = button(&mut app, MenuAction::NewGame);
    move_cursor(&mut app, center);
    for phase in [TouchPhase::Moved, TouchPhase::Ended] {
        let before = camera(&mut app);
        mouse_scroll_with_phase(&mut app, MouseScrollUnit::Pixel, -30.0, phase);
        app.update();
        assert_ne!(
            camera(&mut app),
            before,
            "crossing a control must retain backdrop ownership"
        );
    }
    let before = camera(&mut app);
    mouse_scroll(&mut app, MouseScrollUnit::Line, -1.0);
    app.update();
    assert_eq!(
        camera(&mut app),
        before,
        "a new wheel tick over the control belongs to the UI"
    );
}

#[test]
fn new_pointer_presses_after_focus_returns_work_in_the_same_batch() {
    use bevy::input::touch::TouchPhase;
    let mut app = launch_logged_app(1.0);
    let (_, new_game) = button(&mut app, MenuAction::NewGame);
    let (_, load_game) = button(&mut app, MenuAction::LoadGame);
    move_cursor(&mut app, new_game);
    mouse_button(&mut app, ButtonState::Pressed);
    touch(&mut app, 1, TouchPhase::Started, new_game);
    window_focus(&mut app, false);
    window_focus(&mut app, true);
    mouse_button(&mut app, ButtonState::Released);
    touch(&mut app, 1, TouchPhase::Ended, new_game);
    move_cursor(&mut app, load_game);
    mouse_button(&mut app, ButtonState::Pressed);
    mouse_button(&mut app, ButtonState::Released);
    touch(&mut app, 2, TouchPhase::Started, load_game);
    touch(&mut app, 2, TouchPhase::Ended, load_game);
    app.update();
    assert_eq!(
        app.world().resource::<Activations>().0,
        [MenuAction::LoadGame, MenuAction::LoadGame]
    );
    app.update();
    assert_eq!(
        app.world().resource::<Activations>().0.len(),
        2,
        "native and direct input must not activate twice"
    );
}

#[test]
fn batched_tab_then_enter_activates_the_new_focus() {
    use bevy::input_focus::InputFocus;
    let mut app = launch_logged_app(1.0);
    let (quit, _) = button(&mut app, MenuAction::Quit);
    for _ in 0..4 {
        key(&mut app, KeyCode::Tab, ButtonState::Pressed, false);
        app.update();
        key(&mut app, KeyCode::Tab, ButtonState::Released, false);
        app.update();
    }
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(quit));
    key(&mut app, KeyCode::Tab, ButtonState::Pressed, false);
    key(&mut app, KeyCode::Tab, ButtonState::Released, false);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
    app.update();
    let (new_game, _) = button(&mut app, MenuAction::NewGame);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(new_game));
    assert_eq!(
        app.world().resource::<Activations>().0,
        [MenuAction::NewGame]
    );
    assert!(app.world().resource::<Messages<AppExit>>().is_empty());
}

#[test]
fn keyboard_activation_precedes_a_later_pointer_focus_change_in_the_same_batch() {
    use bevy::input::touch::TouchPhase;
    let mut app = launch_logged_app(1.0);
    key(&mut app, KeyCode::Tab, ButtonState::Pressed, false);
    app.update();
    key(&mut app, KeyCode::Tab, ButtonState::Released, false);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
    let (_, load_game) = button(&mut app, MenuAction::LoadGame);
    touch(&mut app, 1, TouchPhase::Started, load_game);
    touch(&mut app, 1, TouchPhase::Ended, load_game);
    app.update();
    assert_eq!(
        app.world().resource::<Activations>().0,
        [MenuAction::NewGame, MenuAction::LoadGame]
    );
}

#[test]
fn keys_received_while_unfocused_cannot_activate_a_later_pointer_target() {
    use bevy::input::touch::TouchPhase;
    let mut app = launch_logged_app(1.0);
    window_focus(&mut app, false);
    key(&mut app, KeyCode::Enter, ButtonState::Pressed, false);
    window_focus(&mut app, true);
    let (_, center) = button(&mut app, MenuAction::NewGame);
    touch(&mut app, 1, TouchPhase::Started, center);
    touch(&mut app, 1, TouchPhase::Ended, center);
    app.update();
    assert_eq!(
        app.world().resource::<Activations>().0,
        [MenuAction::NewGame]
    );
}
