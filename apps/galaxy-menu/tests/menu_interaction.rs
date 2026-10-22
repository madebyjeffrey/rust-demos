mod common;

use bevy::{
    input::{ButtonState, mouse::MouseScrollUnit},
    prelude::*,
    ui::UiGlobalTransform,
};
use common::{launch_app, mouse_button, mouse_motion, mouse_scroll, point_cursor};
use galaxy_menu::MenuAction;

#[test]
fn pointing_and_clicking_the_backdrop_keeps_only_startup_menu_text() {
    for scale in [1.0, 2.0] {
        let mut app = launch_app(
            UVec2::new((1280.0 * scale) as u32, (720.0 * scale) as u32),
            scale,
        );
        for cursor in [Vec2::new(640.0, 360.0), Vec2::new(900.0, 200.0)] {
            point_cursor(&mut app, cursor, false);
            point_cursor(&mut app, cursor, true);
            let world = app.world_mut();
            let mut labels = world
                .query::<&Text>()
                .iter(world)
                .map(|text| text.0.as_str())
                .collect::<Vec<_>>();
            labels.sort_unstable();
            assert_eq!(
                labels,
                [
                    "CHOOSE YOUR NEXT STEP",
                    "GALAXY MENU",
                    "Load Game",
                    "New Game",
                    "Quit",
                    "Settings",
                ],
                "hovering or clicking must not present destination labels or a details card"
            );
        }
    }
}

#[test]
fn menu_whitespace_remains_available_for_backdrop_orbit_and_zoom() {
    for scale in [1.0, 2.0] {
        let mut app = launch_app(
            UVec2::new((1280.0 * scale) as u32, (720.0 * scale) as u32),
            scale,
        );
        let cursor = Vec2::new(425.0, 225.0);
        let world = app.world_mut();
        assert!(
            world
                .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<MenuAction>>()
                .iter(world)
                .all(|(node, transform)| !node.contains_point(*transform, cursor * scale)),
            "the cursor is in unused space beside the menu actions"
        );
        point_cursor(&mut app, cursor, false);
        let before = camera_transform(&mut app);
        mouse_button(&mut app, ButtonState::Pressed);
        mouse_motion(&mut app, Vec2::new(70.0, 20.0));
        app.update();
        let orbited = camera_transform(&mut app);
        assert!(before.rotation.angle_between(orbited.rotation) > 0.1);

        mouse_button(&mut app, ButtonState::Released);
        mouse_scroll(&mut app, MouseScrollUnit::Line, -1.0);
        app.update();
        let zoomed = camera_transform(&mut app);
        assert!(
            (zoomed.translation - orbited.translation).dot(*orbited.forward()) < -0.1,
            "wheel input in menu whitespace should zoom out"
        );
    }
}

#[test]
fn menu_overlays_preserve_full_window_framing_and_the_camera_view() {
    for logical_size in [UVec2::new(1280, 720), UVec2::new(720, 900)] {
        for scale in [1.0, 2.0] {
            let mut app = launch_app(logical_size * scale as u32, scale);
            let before = camera_transform(&mut app);
            let world = app.world_mut();
            let mut root_nodes = world.query_filtered::<(Entity, &mut Node), Without<ChildOf>>();
            let roots = root_nodes
                .iter_mut(world)
                .map(|(entity, mut node)| {
                    let display = node.display;
                    node.display = Display::None;
                    (entity, display)
                })
                .collect::<Vec<_>>();
            assert!(!roots.is_empty(), "the assembled menu must be present");
            app.update();
            assert_eq!(camera_transform(&mut app), before);

            for (entity, display) in roots {
                app.world_mut().get_mut::<Node>(entity).unwrap().display = display;
            }
            app.update();
            assert_eq!(camera_transform(&mut app), before);
            let world = app.world_mut();
            let camera = world
                .query_filtered::<&Camera, With<Camera3d>>()
                .single(world)
                .unwrap();
            assert_eq!(
                camera.logical_viewport_rect(),
                Some(Rect::from_corners(Vec2::ZERO, logical_size.as_vec2())),
                "menu overlays must leave the whole window available to the backdrop"
            );
        }
    }
}

fn camera_transform(app: &mut App) -> Transform {
    let world = app.world_mut();
    *world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .unwrap()
}
