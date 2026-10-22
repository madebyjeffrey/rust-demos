use bevy::prelude::*;
use bevy_composable_ui::{Anchor, AxisAnchor, FrameError, Rail, SharedFrame};

fn point(x: f32, y: f32) -> Anchor {
    Anchor::frame(0., 0.).offset(x, y)
}

#[test]
fn circular_bands_have_transparent_inner_corners_in_all_four_orientations() {
    for (flip_x, flip_y) in [(false, false), (true, false), (false, true), (true, true)] {
        let flip = |p: Vec2| {
            Vec2::new(
                if flip_x { 100. - p.x } else { p.x },
                if flip_y { 100. - p.y } else { p.y },
            )
        };
        let anchors = [
            Vec2::new(20., 80.),
            Vec2::new(20., 20.),
            Vec2::new(80., 20.),
        ]
        .map(|p| {
            let p = flip(p);
            point(p.x, p.y)
        });
        let frame = SharedFrame::new([Rail::new(anchors, 8., Color::WHITE).rounded(20.)]);
        let geometry = frame.resolve(Vec2::splat(100.), |_| None).unwrap();
        assert!(geometry.contains(flip(Vec2::new(25.86, 25.86))));
        assert!(!geometry.contains(flip(Vec2::new(31., 31.))));
        assert!(geometry.contains(flip(Vec2::new(20., 65.))));
        assert!(!geometry.contains(flip(Vec2::new(10., 65.))));
    }
}

#[test]
fn corridor_midpoints_follow_resolved_node_edges() {
    let mut world = World::new();
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    let x = AxisAnchor::Between((a, 1.), (b, 0.));
    let frame = SharedFrame::new([Rail::new(
        [
            Anchor::at(x, AxisAnchor::Frame(0.)),
            Anchor::at(x, AxisAnchor::Frame(1.)),
        ],
        4.,
        Color::WHITE,
    )]);
    for gap_start in [140., 240.] {
        let geometry = frame
            .resolve(Vec2::splat(500.), |entity| {
                if entity == a {
                    Some(Rect::new(0., 0., 100., 100.))
                } else if entity == b {
                    Some(Rect::new(gap_start, 0., 400., 100.))
                } else {
                    None
                }
            })
            .unwrap();
        assert!(geometry.contains(Vec2::new((100. + gap_start) / 2., 200.)));
        assert!(!geometry.contains(Vec2::new(100., 200.)));
    }
    assert_eq!(
        frame.resolve(Vec2::splat(500.), |_| None),
        Err(FrameError::MissingNode(a))
    );
}

#[test]
fn frame_relative_routes_resize_without_replacing_the_description() {
    let frame = SharedFrame::new([Rail::new(
        [Anchor::frame(0., 1.), Anchor::frame(1., 1.)],
        2.,
        Color::WHITE,
    )]);
    let small = frame.resolve(Vec2::new(100., 80.), |_| None).unwrap();
    let large = frame.resolve(Vec2::new(300., 200.), |_| None).unwrap();
    assert!(small.contains(Vec2::new(50., 80.)));
    assert!(!small.contains(Vec2::new(50., 200.)));
    assert!(large.contains(Vec2::new(250., 200.)));
}

#[test]
fn zero_width_and_empty_frames_have_no_coverage() {
    let frame = SharedFrame::new([Rail::new(
        [point(10., 10.), point(90., 10.)],
        0.,
        Color::WHITE,
    )]);
    let geometry = frame.resolve(Vec2::splat(100.), |_| None).unwrap();
    assert_eq!(geometry.segment_count(), 0);
    assert!(!geometry.contains(Vec2::new(10., 10.)));
    assert_eq!(
        SharedFrame::new([])
            .resolve(Vec2::splat(100.), |_| None)
            .unwrap()
            .segment_count(),
        0
    );
}

#[test]
fn invalid_routes_and_impossible_bends_fail_instead_of_partially_rendering() {
    let resolve = |rail| SharedFrame::new([rail]).resolve(Vec2::splat(100.), |_| None);
    assert_eq!(
        resolve(Rail::new(
            [point(0., 0.), point(50., 50.)],
            2.,
            Color::WHITE
        )),
        Err(FrameError::NonOrthogonal)
    );
    assert_eq!(
        resolve(
            Rail::new(
                [point(0., 20.), point(0., 0.), point(20., 0.)],
                30.,
                Color::WHITE
            )
            .rounded(20.)
        ),
        Err(FrameError::RadiusTooTight)
    );
    assert_eq!(
        resolve(Rail::new(
            [point(0., 0.), point(50., 0.)],
            f32::NAN,
            Color::WHITE
        )),
        Err(FrameError::InvalidNumber)
    );
    assert_eq!(
        resolve(Rail::new([point(0., 0.)], 2., Color::WHITE)),
        Err(FrameError::TooFewPoints)
    );
}

#[test]
fn geometry_budget_is_atomic_and_explicit() {
    let rails = (0..33).map(|i| {
        Rail::new(
            [point(0., i as f32), point(100., i as f32)],
            1.,
            Color::WHITE,
        )
    });
    assert_eq!(
        SharedFrame::new(rails).resolve(Vec2::splat(100.), |_| None),
        Err(FrameError::TooComplex)
    );
}
