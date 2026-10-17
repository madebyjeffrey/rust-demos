mod common;

use bevy::{prelude::*, text::TextLayoutInfo, ui::UiGlobalTransform};
use common::{launch_app, set_viewport};
use galaxy_menu::{MenuAction, WindowSafeArea};

#[test]
fn controls_reflow_inside_narrow_short_rotated_and_multitasking_windows() {
    for scale in [1.0, 2.0, 3.0] {
        let mut app = launch_app(UVec2::new(1280, 720) * scale as u32, scale);
        for logical in [
            UVec2::new(320, 200),
            UVec2::new(360, 160),
            UVec2::new(280, 160),
            UVec2::new(240, 200),
            UVec2::new(240, 600),
            UVec2::new(600, 100),
            UVec2::new(240, 240),
            UVec2::new(280, 240),
            UVec2::new(280, 320),
            UVec2::new(1200, 240),
            UVec2::new(402, 874),
            UVec2::new(874, 402),
            UVec2::new(744, 1133),
            UVec2::new(1133, 744),
            UVec2::new(320, 744),
            UVec2::new(1600, 900),
        ] {
            set_viewport(&mut app, logical * scale as u32, scale);
            app.update();
            app.update();
            assert_controls_fit(
                &mut app,
                Rect::from_corners(Vec2::ZERO, logical.as_vec2()),
                scale,
            );
        }
    }
}

fn assert_controls_fit(app: &mut App, usable: Rect, scale: f32) {
    let world = app.world_mut();
    let mut bounds = Vec::new();
    for (action, node, transform, font, text, visible) in world
        .query::<(
            &MenuAction,
            &ComputedNode,
            &UiGlobalTransform,
            &TextFont,
            &TextLayoutInfo,
            &InheritedVisibility,
        )>()
        .iter(world)
    {
        assert!(visible.get(), "{action:?} remains visible");
        let size = node.size() / scale;
        let center = transform.translation / scale;
        let rect = Rect::from_center_size(center, size);
        assert!(
            size.x >= 120.0 && size.y >= 44.0,
            "{action:?} target too small: {size}"
        );
        assert!(
            rect.min.x >= usable.min.x - 0.5
                && rect.min.y >= usable.min.y - 0.5
                && rect.max.x <= usable.max.x + 0.5
                && rect.max.y <= usable.max.y + 0.5,
            "{action:?} at {rect:?} must fit {usable:?}"
        );
        assert!(
            matches!(font.font_size, FontSize::Px(size) if size >= 19.0),
            "{action:?} remains readable"
        );
        assert!(
            !text.glyphs.is_empty(),
            "{action:?} has measured visible glyphs"
        );
        assert!(
            text.size.x <= node.content_box().size().x + 0.5
                && text.size.y <= node.content_box().size().y + 0.5,
            "{action:?} label {:?} must fit its content area {:?} in {usable:?}",
            text.size,
            node.content_box().size()
        );
        for previous in &bounds {
            assert!(
                rect.intersect(*previous).is_empty(),
                "controls cannot overlap"
            );
        }
        bounds.push(rect);
    }
    assert_eq!(bounds.len(), if cfg!(target_os = "ios") { 3 } else { 4 });
}

#[test]
fn changing_safe_areas_keeps_controls_inside_the_current_usable_window() {
    for scale in [1.0, 2.0, 3.0] {
        let mut app = launch_app(UVec2::new(402, 874) * scale as u32, scale);
        for (logical, insets) in [
            (
                UVec2::new(402, 874),
                WindowSafeArea {
                    top: 62.0,
                    bottom: 34.0,
                    ..default()
                },
            ),
            (
                UVec2::new(874, 402),
                WindowSafeArea {
                    left: 62.0,
                    right: 62.0,
                    bottom: 21.0,
                    ..default()
                },
            ),
            (
                UVec2::new(320, 744),
                WindowSafeArea {
                    top: 24.0,
                    bottom: 20.0,
                    ..default()
                },
            ),
            (
                UVec2::new(320, 200),
                WindowSafeArea {
                    top: 24.0,
                    bottom: 20.0,
                    ..default()
                },
            ),
            (
                UVec2::new(320, 200),
                WindowSafeArea {
                    left: 20.0,
                    right: 20.0,
                    top: 20.0,
                    bottom: 20.0,
                },
            ),
            (
                UVec2::new(500, 360),
                WindowSafeArea {
                    top: 24.0,
                    left: 48.0,
                    right: 48.0,
                    bottom: 24.0,
                },
            ),
            (
                UVec2::new(402, 874),
                WindowSafeArea {
                    top: 62.0,
                    bottom: 34.0,
                    ..default()
                },
            ),
        ] {
            set_viewport(&mut app, logical * scale as u32, scale);
            let window = common::primary_window(&mut app);
            app.world_mut().entity_mut(window).insert(insets);
            app.update();
            app.update();
            assert_controls_fit(
                &mut app,
                Rect::from_corners(
                    Vec2::new(insets.left, insets.top),
                    logical.as_vec2() - Vec2::new(insets.right, insets.bottom),
                ),
                scale,
            );
        }
    }
}

#[test]
fn repeated_resize_and_rotation_preserve_orbit_and_zoom_relative_to_the_full_window_fit() {
    use bevy::input::{ButtonState, mouse::MouseScrollUnit};
    use common::{mouse_button, mouse_motion, mouse_scroll, point_cursor};
    for scale in [1.0, 2.0] {
        let mut app = launch_app(UVec2::new(1280, 720) * scale as u32, scale);
        let mut fitted = launch_app(UVec2::new(1280, 720) * scale as u32, scale);
        for app in [&mut app, &mut fitted] {
            point_cursor(app, Vec2::new(1100.0, 650.0), false);
            mouse_button(app, ButtonState::Pressed);
            mouse_motion(app, Vec2::new(90.0, -35.0));
            app.update();
            mouse_button(app, ButtonState::Released);
            mouse_scroll(app, MouseScrollUnit::Line, 1000.0);
            app.update();
        }
        mouse_scroll(&mut app, MouseScrollUnit::Line, -2.0);
        app.update();
        let before = camera_transform(&mut app);
        let target = Vec3::new(3.8, 0.0, 0.0);
        let relative_zoom = before.translation.distance(target)
            / camera_transform(&mut fitted).translation.distance(target);
        assert!(
            relative_zoom > 1.1,
            "the user chose a view outside the nearest fit"
        );
        for _ in 0..3 {
            for logical in [
                UVec2::new(402, 874),
                UVec2::new(874, 402),
                UVec2::new(320, 744),
                UVec2::new(1280, 720),
            ] {
                for app in [&mut app, &mut fitted] {
                    set_viewport(app, logical * scale as u32, scale);
                    app.update();
                    app.update();
                }
                let camera = camera_transform(&mut app);
                assert!(
                    before.rotation.angle_between(camera.rotation) < 0.001,
                    "rotation must preserve the user's orbit"
                );
                let observed = camera.translation.distance(target)
                    / camera_transform(&mut fitted).translation.distance(target);
                assert!(
                    (observed - relative_zoom).abs() < 0.001,
                    "{logical:?} lost relative zoom: expected {relative_zoom}, got {observed}"
                );
                let world = app.world_mut();
                let camera = world
                    .query_filtered::<&Camera, With<Camera3d>>()
                    .single(world)
                    .unwrap();
                assert_eq!(
                    camera.logical_viewport_rect(),
                    Some(Rect::from_corners(Vec2::ZERO, logical.as_vec2()))
                );
            }
        }
        assert!(
            before
                .translation
                .distance(camera_transform(&mut app).translation)
                < 0.001,
            "returning to the original size cannot accumulate framing drift"
        );
    }
}

fn camera_transform(app: &mut App) -> Transform {
    let world = app.world_mut();
    *world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .unwrap()
}
