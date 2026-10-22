use bevy::{
    asset::AssetPlugin,
    camera::{CameraPlugin, RenderTargetInfo},
    ecs::world::CommandQueue,
    image::ImagePlugin,
    input::InputPlugin,
    input_focus::{
        FocusCause, InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    mesh::MeshPlugin,
    picking::DefaultPickingPlugins,
    prelude::*,
    text::TextPlugin,
    time::TimeUpdateStrategy,
    ui::{InteractionDisabled, UiPlugin},
    ui_widgets::{Activate, Button},
    window::{PrimaryWindow, WindowPlugin},
};
use bevy_composable_ui::{Key, ViewError, prelude::*, reconcile};
use std::time::Duration;

#[derive(Component)]
struct Local(u32);
#[derive(Resource, Default)]
struct Clicks(u32);

fn spec() -> ViewTransition {
    ViewTransition {
        duration: Duration::from_millis(200),
        enter_offset: Vec2::new(0., 20.),
        exit_offset: Vec2::new(0., -20.),
        edge_scale: 0.5,
    }
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        InputPlugin,
        ImagePlugin::default(),
        MeshPlugin,
        CameraPlugin,
        TextPlugin,
        UiPlugin,
        DefaultPickingPlugins,
        WindowPlugin {
            primary_window: None,
            ..default()
        },
        ReactiveViewsPlugin,
    ))
    .init_asset::<TextureAtlasLayout>()
    .init_resource::<Clicks>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.world_mut().spawn(Camera2d);
    step(&mut app, 0);
    app
}

fn step(app: &mut App, millis: u64) {
    *app.world_mut().resource_mut::<TimeUpdateStrategy>() =
        TimeUpdateStrategy::ManualDuration(Duration::from_millis(millis));
    let mut cameras = app.world_mut().query::<&mut Camera>();
    for mut camera in cameras.iter_mut(app.world_mut()) {
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(600, 400),
            scale_factor: 1.,
        });
    }
    app.update();
}

fn list(ids: &[u64]) -> Element<Vec<ViewNode>> {
    let mut root = row(ids
        .iter()
        .map(|id| {
            let mut child = column(()).fill(Color::WHITE);
            child.node.width = px(20);
            child.node.height = px(10);
            let mut item = column((child,)).fill(Color::BLACK);
            item.node.width = px(80);
            item.node.height = px(40);
            item.tagged((Button, Local(0), TabIndex(0), Pickable::default()))
                .on_activate(|world| world.resource_mut::<Clicks>().0 += 1)
                .key(*id)
                .transition(spec())
        })
        .collect::<Vec<_>>())
    .gap(10.);
    root.node.width = px(400);
    root.node.height = px(100);
    root
}

fn mount(app: &mut App, view: impl View) -> Entity {
    let mut queue = CommandQueue::default();
    let root = view.mount(&mut Commands::new(&mut queue, app.world()));
    queue.apply(app.world_mut());
    step(app, 0);
    root
}

fn item(app: &mut App, id: u64) -> Entity {
    app.world_mut()
        .query_filtered::<(Entity, &ViewKey), Without<ExitingView>>()
        .iter(app.world())
        .find(|(_, key)| key.0 == Key::Number(id))
        .unwrap()
        .0
}
fn position(app: &App, entity: Entity) -> Vec2 {
    app.world()
        .get::<UiGlobalTransform>(entity)
        .unwrap()
        .translation
}
fn phase(app: &App, entity: Entity) -> TransitionPhase {
    app.world().get::<TransitionStatus>(entity).unwrap().phase
}
fn close(a: Vec2, b: Vec2) {
    assert!(a.distance(b) < 0.05, "{a:?} != {b:?}");
}

#[test]
fn insertions_scale_slide_and_settle_in_real_bevy_layout() {
    let mut app = app();
    let root = mount(&mut app, list(&[1, 2]));
    step(&mut app, 200);
    reconcile(app.world_mut(), root, list(&[1, 3, 2])).unwrap();
    step(&mut app, 0);
    let added = item(&mut app, 3);
    assert_eq!(phase(&app, added), TransitionPhase::Entering);
    let start = position(&app, added);
    let scale = app
        .world()
        .get::<UiGlobalTransform>(added)
        .unwrap()
        .to_scale_angle_translation()
        .0;
    close(scale, Vec2::splat(0.5));
    step(&mut app, 100);
    assert!((app.world().get::<TransitionStatus>(added).unwrap().progress - 0.5).abs() < 0.01);
    let middle = position(&app, added);
    step(&mut app, 100);
    assert_eq!(phase(&app, added), TransitionPhase::Idle);
    close(start - position(&app, added), Vec2::new(0., 20.));
    close(middle - position(&app, added), Vec2::new(0., 10.));
    close(
        app.world()
            .get::<UiGlobalTransform>(added)
            .unwrap()
            .to_scale_angle_translation()
            .0,
        Vec2::ONE,
    );
}

#[test]
fn reordering_and_interrupting_motion_preserve_identity_focus_and_continuity() {
    let mut app = app();
    let root = mount(&mut app, list(&[1, 2, 3]));
    step(&mut app, 200);
    let first = item(&mut app, 1);
    app.world_mut().get_mut::<Local>(first).unwrap().0 = 42;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(first, FocusCause::Navigated);
    let before = position(&app, first);
    reconcile(app.world_mut(), root, list(&[3, 2, 1])).unwrap();
    step(&mut app, 0);
    close(position(&app, first), before);
    assert_eq!(phase(&app, first), TransitionPhase::Moving);
    step(&mut app, 100);
    let halfway = position(&app, first);
    assert!(halfway.x > before.x + 20.);
    let child = app.world().get::<Children>(first).unwrap()[0];
    let delta = position(&app, child) - halfway;
    reconcile(app.world_mut(), root, list(&[2, 1, 3, 4])).unwrap();
    step(&mut app, 0);
    close(position(&app, first), halfway);
    close(position(&app, child) - position(&app, first), delta);
    step(&mut app, 200);
    assert_eq!(item(&mut app, 1), first);
    assert_eq!(app.world().get::<Local>(first).unwrap().0, 42);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(first));
    assert_eq!(phase(&app, first), TransitionPhase::Idle);
}

#[test]
fn exits_release_layout_disable_input_and_clear_only_removed_focus() {
    let mut app = app();
    let root = mount(&mut app, list(&[1, 2, 3]));
    step(&mut app, 200);
    let removed = item(&mut app, 2);
    app.world_mut().entity_mut(removed).insert(TabGroup {
        order: 0,
        modal: true,
    });
    let focused = app.world().get::<Children>(removed).unwrap()[0];
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(focused, FocusCause::Navigated);
    let before = position(&app, removed);
    reconcile(app.world_mut(), root, list(&[1, 3])).unwrap();
    assert!(app.world().get::<ExitingView>(removed).is_some());
    assert!(app.world().get::<InteractionDisabled>(removed).is_some());
    assert!(app.world().get::<TabIndex>(removed).is_none());
    assert!(app.world().get::<TabGroup>(removed).is_none());
    assert!(!app.world().get::<Pickable>(removed).unwrap().is_hoverable);
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    assert_eq!(
        app.world().get::<Node>(removed).unwrap().position_type,
        PositionType::Absolute
    );
    assert_eq!(
        reconcile(app.world_mut(), removed, column(()).key(2u64)),
        Err(ViewError::NotMounted(removed))
    );
    app.world_mut().trigger(Activate { entity: removed });
    step(&mut app, 0);
    assert_eq!(app.world().resource::<Clicks>().0, 0);
    close(position(&app, removed), before);
    step(&mut app, 100);
    assert_eq!(phase(&app, removed), TransitionPhase::Exiting);
    assert!(position(&app, removed).y < before.y);
    // Reusing a removed key is a new lifetime, even while the old paint is exiting.
    reconcile(app.world_mut(), root, list(&[1, 2, 3])).unwrap();
    let replacement = item(&mut app, 2);
    assert_ne!(replacement, removed);
    step(&mut app, 100);
    assert!(app.world().get_entity(removed).is_err());
    assert!(app.world().get_entity(replacement).is_ok());
}

#[test]
fn disabling_motion_settles_without_losing_focus_and_removes_pending_exits() {
    let mut app = app();
    let root = mount(&mut app, list(&[1, 2, 3]));
    step(&mut app, 200);
    let focused = item(&mut app, 2);
    let removed = item(&mut app, 3);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(focused, FocusCause::Navigated);
    reconcile(app.world_mut(), root, list(&[2, 1])).unwrap();
    step(&mut app, 50);
    app.world_mut().resource_mut::<TransitionSettings>().enabled = false;
    step(&mut app, 0);
    assert!(app.world().get_entity(removed).is_err());
    assert_eq!(phase(&app, focused), TransitionPhase::Idle);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(focused));
    let settled = position(&app, focused);
    app.world_mut().resource_mut::<TransitionSettings>().enabled = true;
    step(&mut app, 10);
    close(position(&app, focused), settled);
    // An owner despawn also cleans up unfinished exits.
    reconcile(app.world_mut(), root, list(&[1])).unwrap();
    app.world_mut().despawn(root);
    step(&mut app, 50);
    assert!(app.world().get_entity(focused).is_err());
}

#[test]
fn invalid_specs_reject_atomically_and_zero_duration_is_immediate() {
    let mut app = app();
    let root = mount(&mut app, list(&[1]));
    assert_eq!(
        reconcile(app.world_mut(), root, column(()).transition(spec())),
        Err(ViewError::TransitionRequiresKey)
    );
    let invalid = ViewTransition {
        edge_scale: f32::NAN,
        ..spec()
    };
    assert_eq!(
        reconcile(
            app.world_mut(),
            root,
            column(()).key(1u64).transition(invalid)
        ),
        Err(ViewError::InvalidTransition)
    );
    let view = column(()).key(1u64).transition(ViewTransition {
        duration: Duration::ZERO,
        ..spec()
    });
    let root = reconcile(app.world_mut(), root, view).unwrap();
    step(&mut app, 0);
    assert_eq!(phase(&app, root), TransitionPhase::Idle);
}

#[test]
fn nested_motion_composes_and_removed_subscriptions_stop_immediately() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    #[derive(Resource)]
    struct Value(u32);
    let mut app = app();
    app.insert_resource(Value(0));
    let renders = Arc::new(AtomicUsize::new(0));
    let calls = renders.clone();
    let inner = Binding::read(|value: &Value| value.0).view(move |_| {
        calls.fetch_add(1, Ordering::Relaxed);
        let mut child = column(());
        child.node.width = px(20);
        child.node.height = px(10);
        child.key(2u64).transition(spec())
    });
    let mut outer = column((inner,));
    outer.node.width = px(80);
    outer.node.height = px(40);
    let root = mount(&mut app, row((outer.key(1u64).transition(spec()),)));
    let outer = item(&mut app, 1);
    let inner = item(&mut app, 2);
    // At half-time both local scales are 0.75; the displayed child scale composes.
    step(&mut app, 100);
    close(
        app.world()
            .get::<UiGlobalTransform>(inner)
            .unwrap()
            .to_scale_angle_translation()
            .0,
        Vec2::splat(0.75 * 0.75),
    );
    step(&mut app, 100);
    reconcile(app.world_mut(), root, row(())).unwrap();
    app.world_mut().resource_mut::<Value>().0 = 1;
    step(&mut app, 50);
    assert_eq!(renders.load(Ordering::Relaxed), 1);
    assert!(app.world().get::<ExitingView>(inner).is_some());
    assert!(app.world().get::<TransitionStatus>(inner).is_none());
    step(&mut app, 150);
    assert!(app.world().get_entity(outer).is_err());
    assert!(app.world().get_entity(inner).is_err());
}

#[test]
fn repeated_removal_and_reinsertion_do_not_leak_departing_entities() {
    let mut app = app();
    let root = mount(&mut app, list(&[1]));
    step(&mut app, 200);
    for _ in 0..20 {
        reconcile(app.world_mut(), root, list(&[])).unwrap();
        step(&mut app, 10);
        reconcile(app.world_mut(), root, list(&[1])).unwrap();
        step(&mut app, 10);
    }
    step(&mut app, 200);
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<ExitingView>>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(app.world().get::<Children>(root).unwrap().len(), 1);
    assert_eq!(
        app.world_mut()
            .query::<&ViewKey>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn simultaneous_parent_resize_and_keyed_edits_do_not_create_spurious_motion() {
    let mut app = app();
    let root = mount(&mut app, list(&[1, 2, 3]));
    step(&mut app, 200);
    let retained = item(&mut app, 1);
    let before = position(&app, retained);
    let mut wider = list(&[1, 2, 3, 4]);
    wider.node.width = px(500);
    wider.node.height = px(200);
    reconcile(app.world_mut(), root, wider).unwrap();
    step(&mut app, 0);
    close(position(&app, retained), before);
    assert_eq!(phase(&app, retained), TransitionPhase::Idle);
    step(&mut app, 200);
    let removed = item(&mut app, 4);
    let before_exit = position(&app, removed);
    let mut narrower = list(&[1, 2, 3]);
    narrower.node.width = px(300);
    reconcile(app.world_mut(), root, narrower).unwrap();
    step(&mut app, 0);
    close(position(&app, retained), before);
    close(position(&app, removed), before_exit);
}

#[test]
fn moving_views_retarget_when_layout_settles_on_a_later_frame() {
    let mut app = app();
    let root = mount(&mut app, list(&[1, 2, 3]));
    step(&mut app, 200);
    let retained = item(&mut app, 1);
    let before = position(&app, retained);
    let mut intermediate = list(&[3, 2, 1, 4]);
    intermediate.node.width = px(100);
    intermediate.node.flex_wrap = FlexWrap::Wrap;
    reconcile(app.world_mut(), root, intermediate).unwrap();
    step(&mut app, 0);
    close(position(&app, retained), before);
    // Window/target propagation or text measurement can settle after reconciliation.
    // No new key edit is needed for a moving view to follow the corrected destination.
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(500);
    step(&mut app, 0);
    close(position(&app, retained), before);
    step(&mut app, 200);
    close(position(&app, retained), Vec2::new(220., 20.));
}
