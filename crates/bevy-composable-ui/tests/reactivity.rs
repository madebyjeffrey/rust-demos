use bevy::{
    ecs::world::CommandQueue,
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    ui_widgets::{Activate, Button},
};
use bevy_composable_ui::{BindingError, Key, ViewError, prelude::*, reconcile};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource)]
struct Model {
    count: u32,
    unrelated: bool,
    items: Vec<u64>,
}
#[derive(Component)]
struct Local(u32);

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ReactiveViewsPlugin))
        .insert_resource(Model {
            count: 0,
            unrelated: false,
            items: vec![1, 2, 3],
        });
    app
}
fn mount(app: &mut App, view: impl View) -> Entity {
    let mut queue = CommandQueue::default();
    let entity = view.mount(&mut Commands::new(&mut queue, app.world()));
    queue.apply(app.world_mut());
    entity
}
fn item_entities(app: &mut App) -> Vec<(u64, Entity)> {
    app.world_mut()
        .query::<(Entity, &ViewKey)>()
        .iter(app.world())
        .filter_map(|(e, k)| match k.0 {
            Key::Number(n) => Some((n, e)),
            _ => None,
        })
        .collect()
}
fn list(items: &[u64], count: u32) -> impl View + use<> {
    row(items
        .iter()
        .map(|id| {
            text(format!("{id} / {count}"), 12., Color::WHITE)
                .tagged((Button, Local(0), Pickable::default()))
                .key(*id)
        })
        .collect::<Vec<_>>())
}

#[test]
fn writable_lens_updates_views_and_equal_selections_do_not_render() {
    let mut app = app();
    let renders = Arc::new(AtomicUsize::new(0));
    let count = Binding::resource(|m: &Model| m.count, |m: &mut Model, v| m.count = v);
    let calls = renders.clone();
    let root = mount(
        &mut app,
        count.clone().view(move |value| {
            calls.fetch_add(1, Ordering::Relaxed);
            text(value.to_string(), 12., Color::WHITE)
        }),
    );
    let child = app.world().get::<Children>(root).unwrap()[0];
    app.update();
    assert_eq!(renders.load(Ordering::Relaxed), 1);
    app.world_mut().resource_mut::<Model>().unrelated = true;
    app.update();
    assert_eq!(renders.load(Ordering::Relaxed), 1);
    count.update(app.world_mut(), |value| *value += 1).unwrap();
    app.update();
    assert_eq!(app.world().get::<Children>(root).unwrap()[0], child);
    assert_eq!(app.world().get::<Text>(child).unwrap().0, "1");
    assert_eq!(renders.load(Ordering::Relaxed), 2);
    count.set(app.world_mut(), 1).unwrap();
    app.update();
    assert_eq!(renders.load(Ordering::Relaxed), 2);
    assert_eq!(
        count
            .map(|n| n.to_string())
            .set(app.world_mut(), "2".into()),
        Err(BindingError::ReadOnly)
    );
}

#[test]
fn keyed_reorder_and_insertion_preserve_focus_and_native_local_state() {
    let mut app = app();
    let root = mount(
        &mut app,
        Binding::read(|m: &Model| (m.items.clone(), m.count))
            .view(|(items, count)| list(items, *count)),
    );
    let ids = item_entities(&mut app);
    let focused = ids.iter().find(|(key, _)| *key == 2).unwrap().1;
    app.world_mut().get_mut::<Local>(focused).unwrap().0 = 47;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(focused, FocusCause::Navigated);
    app.world_mut().resource_mut::<Model>().items = vec![3, 4, 2, 1];
    app.world_mut().resource_mut::<Model>().count = 8;
    app.update();
    for (key, entity) in ids {
        assert!(item_entities(&mut app).contains(&(key, entity)));
    }
    let row = app.world().get::<Children>(root).unwrap()[0];
    let order: Vec<_> = app
        .world()
        .get::<Children>(row)
        .unwrap()
        .iter()
        .map(|e| app.world().get::<ViewKey>(e).unwrap().0.clone())
        .collect();
    assert_eq!(
        order,
        vec![
            Key::Number(3),
            Key::Number(4),
            Key::Number(2),
            Key::Number(1)
        ]
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(focused));
    assert_eq!(app.world().get::<Local>(focused).unwrap().0, 47);
    assert_eq!(app.world().get::<Text>(focused).unwrap().0, "2 / 8");
}

#[test]
fn removal_clears_focus_and_reinsertion_starts_a_new_lifetime() {
    let mut app = app();
    mount(
        &mut app,
        Binding::read(|m: &Model| m.items.clone()).view(|items| list(items, 0)),
    );
    let focused = item_entities(&mut app)
        .into_iter()
        .find(|(k, _)| *k == 2)
        .unwrap()
        .1;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(focused, FocusCause::Navigated);
    app.world_mut().resource_mut::<Model>().items = vec![1, 3];
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    assert!(app.world().get_entity(focused).is_err());
    app.world_mut().resource_mut::<Model>().items = vec![1, 2, 3];
    app.update();
    assert_ne!(
        item_entities(&mut app)
            .into_iter()
            .find(|(k, _)| *k == 2)
            .unwrap()
            .1,
        focused
    );
}

#[test]
fn duplicate_keys_reject_the_whole_update_and_recover_on_a_new_selection() {
    let mut app = app();
    let root = mount(
        &mut app,
        Binding::read(|m: &Model| m.items.clone()).view(|items| list(items, 0)),
    );
    let old = item_entities(&mut app);
    app.world_mut().resource_mut::<Model>().items = vec![3, 2, 2, 1];
    app.update();
    assert_eq!(item_entities(&mut app), old);
    assert_eq!(
        app.world().get::<ViewDiagnostic>(root).unwrap().0,
        Some(ViewError::DuplicateKey(Key::Number(2)))
    );
    let model = app.world_mut().remove_resource::<Model>().unwrap();
    app.update();
    app.world_mut().insert_resource(model);
    app.update();
    assert_eq!(
        app.world().get::<ViewDiagnostic>(root).unwrap().0,
        Some(ViewError::DuplicateKey(Key::Number(2)))
    );
    app.world_mut().resource_mut::<Model>().items = vec![3, 2, 1];
    app.update();
    assert!(app.world().get::<ViewDiagnostic>(root).unwrap().0.is_none());
}

#[test]
fn incompatible_kinds_replace_entities_and_clear_descendant_focus() {
    let mut app = app();
    let root = mount(&mut app, row((text("A", 12., Color::WHITE).key("item"),)));
    let old = app.world().get::<Children>(root).unwrap()[0];
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(old, FocusCause::Navigated);
    reconcile(app.world_mut(), root, row((column(()).key("item"),))).unwrap();
    let next = app.world().get::<Children>(root).unwrap()[0];
    assert_ne!(old, next);
    assert!(app.world().get_entity(old).is_err());
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    let result = reconcile(
        app.world_mut(),
        root,
        row((
            text("X", 12., Color::WHITE).key("x"),
            text("Y", 12., Color::WHITE).key("x"),
        )),
    );
    assert_eq!(result, Err(ViewError::DuplicateKey("x".into())));
    assert_eq!(app.world().get::<Children>(root).unwrap()[0], next);
}

#[test]
fn replacing_a_handler_uses_current_captures_without_remounting() {
    let mut app = app();
    let root = mount(
        &mut app,
        Binding::read(|m: &Model| m.count).view(|count| {
            let next = *count + 1;
            text("Increment", 12., Color::WHITE)
                .tagged((Button, Pickable::default()))
                .on_activate(move |world| world.resource_mut::<Model>().count = next)
        }),
    );
    let button = app.world().get::<Children>(root).unwrap()[0];
    for expected in [1, 2, 3] {
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        assert_eq!(app.world().resource::<Model>().count, expected);
        assert_eq!(app.world().get::<Children>(root).unwrap()[0], button);
    }
}

#[test]
fn independent_subscriptions_and_missing_resource_recovery() {
    #[derive(Resource)]
    struct Later(u32);
    let mut app = app();
    let binding = Binding::read(|m: &Later| m.0);
    let a = mount(
        &mut app,
        binding
            .clone()
            .view(|n| text(n.to_string(), 12., Color::WHITE)),
    );
    let b = mount(
        &mut app,
        binding.view(|n| text(format!("B{n}"), 12., Color::WHITE)),
    );
    assert!(app.world().get::<ViewDiagnostic>(a).unwrap().0.is_some());
    app.world_mut().insert_resource(Later(9));
    app.update();
    for (root, value) in [(a, "9"), (b, "B9")] {
        assert!(app.world().get::<ViewDiagnostic>(root).unwrap().0.is_none());
        let child = app.world().get::<Children>(root).unwrap()[0];
        assert_eq!(app.world().get::<Text>(child).unwrap().0, value);
    }
    app.world_mut().despawn(a);
    app.world_mut().resource_mut::<Later>().0 = 10;
    app.update();
    assert!(app.world().get_entity(a).is_err());
    let child = app.world().get::<Children>(b).unwrap()[0];
    assert_eq!(app.world().get::<Text>(child).unwrap().0, "B10");
    app.world_mut().remove_resource::<Later>();
    app.update();
    assert!(app.world().get::<ViewDiagnostic>(b).unwrap().0.is_some());
    app.world_mut().insert_resource(Later(10));
    app.update();
    assert!(app.world().get::<ViewDiagnostic>(b).unwrap().0.is_none());
}

#[test]
fn invalid_initial_mount_can_be_replaced_by_a_valid_description() {
    let mut app = app();
    let root = mount(
        &mut app,
        row((
            text("A", 12., Color::WHITE).key("same"),
            text("B", 12., Color::WHITE).key("same"),
        )),
    );
    assert!(app.world().get::<ViewDiagnostic>(root).unwrap().0.is_some());
    let next = reconcile(app.world_mut(), root, text("Recovered", 12., Color::WHITE)).unwrap();
    assert_eq!(app.world().get::<Text>(next).unwrap().0, "Recovered");
}

#[test]
fn recursive_bindings_remain_bounded_across_subsequent_frames() {
    fn recursive() -> bevy_composable_ui::BoundView {
        Binding::read(|model: &Model| model.count).view(|_| recursive())
    }
    let mut app = app();
    mount(&mut app, recursive());
    let count = app
        .world_mut()
        .query::<&ViewDiagnostic>()
        .iter(app.world())
        .count();
    assert_eq!(count, 64);
    for _ in 0..4 {
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&ViewDiagnostic>()
                .iter(app.world())
                .count(),
            count
        );
    }
    assert!(
        app.world_mut()
            .query::<&ViewDiagnostic>()
            .iter(app.world())
            .any(|status| status.0 == Some(ViewError::TooDeep))
    );
}
