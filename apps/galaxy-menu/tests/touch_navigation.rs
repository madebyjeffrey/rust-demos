mod common;

use bevy::{input::touch::TouchPhase, prelude::*};
use common::{launch_app, touch};

fn camera(app: &mut App) -> Transform {
    let world = app.world_mut();
    *world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .unwrap()
}

#[test]
fn one_finger_drag_orbits_the_backdrop_without_repeating_held_movement() {
    for scale in [1.0, 2.0] {
        let mut app = launch_app(UVec2::new(1280, 720) * scale as u32, scale);
        let before = camera(&mut app);
        touch(&mut app, 1, TouchPhase::Started, Vec2::new(600.0, 300.0));
        app.update();
        assert_eq!(
            camera(&mut app),
            before,
            "touch down only establishes an anchor"
        );
        touch(&mut app, 1, TouchPhase::Moved, Vec2::new(670.0, 320.0));
        app.update();
        let orbited = camera(&mut app);
        assert!(
            before.rotation.angle_between(orbited.rotation) > 0.1,
            "dragging must orbit the camera"
        );
        app.update();
        assert_eq!(
            camera(&mut app),
            orbited,
            "holding still cannot replay the last movement"
        );
    }
}

#[test]
fn pinching_changes_zoom_without_orbit_and_stops_at_the_existing_bounds() {
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    touch(&mut app, 1, TouchPhase::Started, Vec2::new(600.0, 300.0));
    touch(&mut app, 2, TouchPhase::Started, Vec2::new(800.0, 300.0));
    app.update();
    let before = camera(&mut app);
    touch(&mut app, 2, TouchPhase::Moved, Vec2::new(700.0, 300.0));
    app.update();
    let farther = camera(&mut app);
    assert!(
        (farther.translation - before.translation).dot(*before.forward()) < -1.0,
        "closing the pinch must zoom out"
    );
    assert_eq!(farther.rotation, before.rotation, "a pinch cannot orbit");
    touch(&mut app, 2, TouchPhase::Moved, Vec2::new(650.0, 300.0));
    app.update();
    let far_limit = camera(&mut app);
    touch(&mut app, 2, TouchPhase::Moved, Vec2::new(625.0, 300.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        far_limit,
        "zoom out must stop at the far bound"
    );
    touch(&mut app, 2, TouchPhase::Moved, Vec2::new(800.0, 300.0));
    app.update();
    let close_limit = camera(&mut app);
    assert!(
        (close_limit.translation - far_limit.translation).dot(*before.forward()) > 1.0,
        "spreading the pinch must zoom in"
    );
    touch(&mut app, 2, TouchPhase::Moved, Vec2::new(1000.0, 300.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        close_limit,
        "zoom in must keep the fitted backdrop visible"
    );
}

#[test]
fn touch_navigation_pauses_idle_galaxy_motion_until_the_gesture_finishes() {
    use bevy::time::TimeUpdateStrategy;
    use galaxy_menu::GalaxyRoot;
    use std::time::Duration;

    fn galaxy_rotation(app: &mut App) -> Quat {
        let world = app.world_mut();
        world
            .query_filtered::<&Transform, With<GalaxyRoot>>()
            .single(world)
            .unwrap()
            .rotation
    }
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    let before = galaxy_rotation(&mut app);
    touch(&mut app, 1, TouchPhase::Started, Vec2::new(600.0, 300.0));
    app.update();
    app.update();
    assert_eq!(
        galaxy_rotation(&mut app),
        before,
        "holding a backdrop touch must pause idle motion"
    );
    touch(&mut app, 1, TouchPhase::Moved, Vec2::new(670.0, 320.0));
    app.update();
    assert_eq!(
        galaxy_rotation(&mut app),
        before,
        "idle motion cannot compete with touch orbit"
    );
    touch(&mut app, 1, TouchPhase::Ended, Vec2::new(670.0, 320.0));
    app.update();
    app.update();
    assert!(
        before.angle_between(galaxy_rotation(&mut app)) > 0.001,
        "idle motion must resume after release"
    );
}

#[test]
fn focus_interruption_discards_old_touches_even_when_focus_returns_between_frames() {
    use common::window_focus;
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    touch(&mut app, 1, TouchPhase::Started, Vec2::new(600.0, 300.0));
    touch(&mut app, 2, TouchPhase::Started, Vec2::new(800.0, 300.0));
    app.update();
    let before = camera(&mut app);
    window_focus(&mut app, false);
    window_focus(&mut app, true);
    touch(&mut app, 1, TouchPhase::Moved, Vec2::new(700.0, 350.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        before,
        "stale input cannot resume an interrupted pinch"
    );
    touch(&mut app, 1, TouchPhase::Started, Vec2::new(700.0, 350.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        before,
        "the new gesture must establish a fresh anchor"
    );
    touch(&mut app, 1, TouchPhase::Moved, Vec2::new(750.0, 350.0));
    app.update();
    assert!(
        before.rotation.angle_between(camera(&mut app).rotation) > 0.1,
        "the next orbit must work without a missing old finger"
    );
}

#[test]
fn suspension_clears_gestures_and_rejects_touch_input_until_the_app_resumes() {
    use bevy::window::{AppLifecycle, WindowEvent};
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    touch(&mut app, 1, TouchPhase::Started, Vec2::new(600.0, 300.0));
    app.update();
    let before = camera(&mut app);
    app.world_mut()
        .write_message(WindowEvent::AppLifecycle(AppLifecycle::WillSuspend));
    touch(&mut app, 1, TouchPhase::Moved, Vec2::new(700.0, 350.0));
    touch(&mut app, 2, TouchPhase::Started, Vec2::new(800.0, 300.0));
    touch(&mut app, 2, TouchPhase::Moved, Vec2::new(900.0, 350.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        before,
        "input received while suspended cannot move the camera"
    );
    app.world_mut()
        .write_message(WindowEvent::AppLifecycle(AppLifecycle::WillResume));
    app.world_mut()
        .write_message(WindowEvent::AppLifecycle(AppLifecycle::Running));
    touch(&mut app, 1, TouchPhase::Moved, Vec2::new(750.0, 350.0));
    touch(&mut app, 2, TouchPhase::Moved, Vec2::new(950.0, 350.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        before,
        "resuming does not reclaim old fingers"
    );
    touch(&mut app, 1, TouchPhase::Started, Vec2::new(750.0, 350.0));
    touch(&mut app, 1, TouchPhase::Moved, Vec2::new(800.0, 350.0));
    app.update();
    assert!(
        before.rotation.angle_between(camera(&mut app).rotation) > 0.1,
        "a fresh touch after resume must orbit normally"
    );
}

#[test]
fn adding_and_removing_fingers_rebases_orbit_and_pinch_without_jumps() {
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    touch(&mut app, 1, TouchPhase::Started, Vec2::new(600.0, 300.0));
    touch(&mut app, 1, TouchPhase::Moved, Vec2::new(650.0, 300.0));
    app.update();
    let orbited = camera(&mut app);
    touch(&mut app, 2, TouchPhase::Started, Vec2::new(900.0, 500.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        orbited,
        "the second finger cannot move the camera on arrival"
    );
    touch(&mut app, 2, TouchPhase::Moved, Vec2::new(800.0, 500.0));
    app.update();
    let pinched = camera(&mut app);
    assert_ne!(
        pinched.translation, orbited.translation,
        "the new pinch must respond"
    );
    assert_eq!(pinched.rotation, orbited.rotation);
    touch(&mut app, 2, TouchPhase::Ended, Vec2::new(1200.0, 700.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        pinched,
        "removing a finger must not change zoom or orbit"
    );
    touch(&mut app, 1, TouchPhase::Moved, Vec2::new(655.0, 300.0));
    app.update();
    let resumed = camera(&mut app);
    let angle = pinched.rotation.angle_between(resumed.rotation);
    assert!(
        angle > 0.001 && angle < 0.05,
        "the remaining finger resumes from its latest position: {angle}"
    );
    touch(&mut app, 1, TouchPhase::Ended, Vec2::new(655.0, 300.0));
    app.update();
    touch(&mut app, 1, TouchPhase::Moved, Vec2::new(900.0, 600.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        resumed,
        "late movement after completion must be ignored"
    );
    touch(&mut app, 1, TouchPhase::Started, Vec2::new(1000.0, 600.0));
    app.update();
    assert_eq!(
        camera(&mut app),
        resumed,
        "reused finger IDs start a new anchor"
    );
}

#[test]
fn cancelling_either_pinch_finger_allows_the_remaining_finger_to_orbit() {
    for cancelled in [1, 2] {
        let mut app = launch_app(UVec2::new(1280, 720), 1.0);
        touch(&mut app, 1, TouchPhase::Started, Vec2::new(600.0, 300.0));
        touch(&mut app, 2, TouchPhase::Started, Vec2::new(800.0, 300.0));
        touch(&mut app, 2, TouchPhase::Moved, Vec2::new(700.0, 300.0));
        app.update();
        let pinched = camera(&mut app);
        touch(
            &mut app,
            cancelled,
            TouchPhase::Canceled,
            Vec2::new(1200.0, 700.0),
        );
        app.update();
        assert_eq!(
            camera(&mut app),
            pinched,
            "cancellation cannot apply a final movement"
        );
        let remaining = if cancelled == 1 { 2 } else { 1 };
        let remaining_position = if remaining == 1 {
            Vec2::new(605.0, 300.0)
        } else {
            Vec2::new(705.0, 300.0)
        };
        touch(&mut app, remaining, TouchPhase::Moved, remaining_position);
        app.update();
        let angle = pinched.rotation.angle_between(camera(&mut app).rotation);
        assert!(
            angle > 0.001 && angle < 0.05,
            "cancelling either finger must rebase the surviving touch: {angle}"
        );
        touch(
            &mut app,
            remaining,
            TouchPhase::Canceled,
            remaining_position,
        );
        app.update();
        let before = camera(&mut app);
        touch(&mut app, 3, TouchPhase::Started, Vec2::new(900.0, 300.0));
        touch(&mut app, 3, TouchPhase::Moved, Vec2::new(950.0, 300.0));
        app.update();
        assert!(
            before.rotation.angle_between(camera(&mut app).rotation) > 0.1,
            "a new gesture must work after all fingers cancel"
        );
    }
}

#[test]
fn touch_navigation_pauses_companion_motion_until_release() {
    use bevy::time::TimeUpdateStrategy;
    use galaxy_menu::GalaxyCompanion;
    use std::time::Duration;
    fn companions(app: &mut App) -> Vec<(Entity, Vec3)> {
        let world = app.world_mut();
        let mut positions = world
            .query_filtered::<(Entity, &Transform), With<GalaxyCompanion>>()
            .iter(world)
            .map(|(entity, transform)| (entity, transform.translation))
            .collect::<Vec<_>>();
        positions.sort_by_key(|(entity, _)| *entity);
        positions
    }
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    let before = companions(&mut app);
    assert!(!before.is_empty(), "the backdrop must include companions");
    touch(&mut app, 1, TouchPhase::Started, Vec2::new(600.0, 300.0));
    touch(&mut app, 2, TouchPhase::Started, Vec2::new(800.0, 300.0));
    app.update();
    touch(&mut app, 2, TouchPhase::Moved, Vec2::new(700.0, 300.0));
    app.update();
    app.update();
    assert_eq!(
        companions(&mut app),
        before,
        "idle companions cannot move the fitted backdrop during a pinch"
    );
    touch(&mut app, 1, TouchPhase::Ended, Vec2::new(600.0, 300.0));
    touch(&mut app, 2, TouchPhase::Ended, Vec2::new(700.0, 300.0));
    app.update();
    assert_ne!(
        companions(&mut app),
        before,
        "companion idle motion resumes on release"
    );
}

#[test]
fn controls_keep_their_touches_while_surrounding_backdrop_gestures_remain_available() {
    use bevy::ui::UiGlobalTransform;
    use galaxy_menu::MenuAction;
    for scale in [1.0, 2.0] {
        let mut app = launch_app(UVec2::new(1280, 720) * scale as u32, scale);
        let center = {
            let world = app.world_mut();
            world
                .query::<(&MenuAction, &UiGlobalTransform)>()
                .iter(world)
                .find(|(action, _)| **action == MenuAction::NewGame)
                .unwrap()
                .1
                .translation
                / scale
        };
        let before = camera(&mut app);
        touch(&mut app, 1, TouchPhase::Started, center);
        touch(&mut app, 2, TouchPhase::Started, center);
        app.update();
        touch(&mut app, 1, TouchPhase::Moved, Vec2::new(700.0, 300.0));
        touch(&mut app, 2, TouchPhase::Moved, Vec2::new(1000.0, 600.0));
        app.update();
        assert_eq!(
            camera(&mut app),
            before,
            "touches beginning on a control cannot orbit or form a pinch after dragging outside"
        );
        touch(&mut app, 1, TouchPhase::Canceled, Vec2::new(700.0, 300.0));
        touch(&mut app, 2, TouchPhase::Canceled, Vec2::new(1000.0, 600.0));
        app.update();
        touch(&mut app, 1, TouchPhase::Started, Vec2::new(425.0, 225.0));
        touch(&mut app, 1, TouchPhase::Moved, center);
        app.update();
        let orbited = camera(&mut app);
        assert!(
            before.rotation.angle_between(orbited.rotation) > 0.1,
            "a backdrop touch keeps ownership when crossing a control"
        );
        touch(&mut app, 2, TouchPhase::Started, center);
        touch(&mut app, 2, TouchPhase::Moved, Vec2::new(1000.0, 600.0));
        app.update();
        assert_eq!(
            camera(&mut app),
            orbited,
            "a UI finger cannot join a backdrop pinch"
        );
        touch(&mut app, 3, TouchPhase::Started, Vec2::new(900.0, 600.0));
        app.update();
        assert_eq!(camera(&mut app), orbited);
        touch(&mut app, 3, TouchPhase::Moved, Vec2::new(700.0, 600.0));
        app.update();
        assert!(
            (camera(&mut app).translation - orbited.translation).dot(*orbited.forward()) < -0.1,
            "two genuine backdrop fingers can pinch while a control holds its own touch"
        );
    }
}

#[test]
fn touch_orbit_remains_continuous_through_the_poles_like_desktop_dragging() {
    use bevy::input::ButtonState;
    use common::{mouse_button, mouse_motion};
    let mut touch_app = launch_app(UVec2::new(1280, 720), 1.0);
    let mut desktop_app = launch_app(UVec2::new(1280, 720), 1.0);
    let mut position = Vec2::new(600.0, 300.0);
    touch(&mut touch_app, 1, TouchPhase::Started, position);
    touch_app.update();
    mouse_button(&mut desktop_app, ButtonState::Pressed);
    for _ in 0..20 {
        let before = camera(&mut touch_app);
        let delta = Vec2::new(0.0, 80.0);
        position += delta;
        touch(&mut touch_app, 1, TouchPhase::Moved, position);
        mouse_motion(&mut desktop_app, delta);
        touch_app.update();
        desktop_app.update();
        let touched = camera(&mut touch_app);
        let angle = before.rotation.angle_between(touched.rotation);
        assert!(
            angle > 0.1 && angle < 0.6,
            "orbiting must keep moving smoothly through every pole: {angle}"
        );
        assert!(
            touched
                .rotation
                .angle_between(camera(&mut desktop_app).rotation)
                < 0.001,
            "touch and retained mouse input must produce the same continuous orbit"
        );
    }
}

#[test]
fn holding_a_menu_control_pauses_idle_motion_like_a_desktop_mouse_press() {
    use bevy::{time::TimeUpdateStrategy, ui::UiGlobalTransform};
    use galaxy_menu::{GalaxyRoot, MenuAction};
    use std::time::Duration;
    fn root_rotation(app: &mut App) -> Quat {
        let world = app.world_mut();
        world
            .query_filtered::<&Transform, With<GalaxyRoot>>()
            .single(world)
            .unwrap()
            .rotation
    }
    let mut app = launch_app(UVec2::new(1280, 720), 1.0);
    let center = {
        let world = app.world_mut();
        world
            .query::<(&MenuAction, &UiGlobalTransform)>()
            .iter(world)
            .find(|(action, _)| **action == MenuAction::NewGame)
            .unwrap()
            .1
            .translation
    };
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    let before = root_rotation(&mut app);
    touch(&mut app, 1, TouchPhase::Started, center);
    app.update();
    app.update();
    assert_eq!(
        root_rotation(&mut app),
        before,
        "UI touch ownership should retain the existing direct-input pause"
    );
    touch(&mut app, 1, TouchPhase::Canceled, center);
    app.update();
    assert!(
        before.angle_between(root_rotation(&mut app)) > 0.001,
        "cancelling a UI touch releases the idle pause"
    );
}
