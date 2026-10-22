use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::FocusedInput,
    prelude::*,
    ui::Checked,
    ui_widgets::{RadioGroupPlugin, ValueChange},
};
use lcars_ui::{LcarsPlugin, Progress, Screen, Tone, Widget};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Choice {
    First,
    Second,
}
#[derive(Resource)]
struct Handles {
    root: Entity,
    group: Entity,
    toggle: Widget,
    progress: Entity,
}
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, bool)>);
fn fixture() -> App {
    let mut app = App::new();
    app.add_plugins((LcarsPlugin, RadioGroupPlugin))
        .init_resource::<Changes>()
        .add_observer(
            |event: On<ValueChange<bool>>, mut changes: ResMut<Changes>| {
                changes.0.push((event.source, event.value));
            },
        )
        .add_systems(Startup, |mut commands: Commands| {
            let mut group = Entity::PLACEHOLDER;
            let mut toggle = Widget {
                entity: Entity::PLACEHOLDER,
                label: Entity::PLACEHOLDER,
            };
            let mut progress = Entity::PLACEHOLDER;
            let screen = Screen::new("TEST", "Public interface").spawn(&mut commands, |ui| {
                ui.column(1.0, |ui| {
                    group = ui.radio_group(
                        Tone::Orange,
                        Choice::First,
                        [(Choice::First, "FIRST"), (Choice::Second, "SECOND")],
                    );
                    toggle = ui.toggle("CHANNEL", Tone::Blue, false, ());
                    progress = ui.progress(0.25, Tone::Blue, ());
                });
            });
            commands.insert_resource(Handles {
                root: screen.root,
                group,
                toggle,
                progress,
            });
        });
    app.update();
    app
}
#[test]
fn radio_keyboard_emits_typed_option_without_mutating_application_state() {
    let mut app = fixture();
    let group = app.world().resource::<Handles>().group;
    let window = app.world_mut().spawn_empty().id();
    app.world_mut().trigger(FocusedInput::new(
        group,
        KeyboardInput {
            key_code: KeyCode::ArrowRight,
            logical_key: Key::ArrowRight,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
        window,
    ));
    app.update();
    let changes = &app.world().resource::<Changes>().0;
    assert_eq!(changes.len(), 1);
    assert!(changes[0].1);
    assert_eq!(
        app.world().get::<Choice>(changes[0].0),
        Some(&Choice::Second)
    );
    assert!(
        app.world().get::<Checked>(changes[0].0).is_none(),
        "controlled widgets wait for the app to accept a value"
    );
}
#[test]
fn bound_entities_update_presentation_and_are_owned_by_screen() {
    let mut app = fixture();
    let handles = app.world().resource::<Handles>();
    let (root, toggle, progress) = (handles.root, handles.toggle, handles.progress);
    let before = app.world().get::<BackgroundColor>(toggle.entity).unwrap().0;
    app.world_mut().entity_mut(toggle.entity).insert(Checked);
    app.world_mut().get_mut::<Text>(toggle.label).unwrap().0 = "RENAMED".into();
    app.world_mut().get_mut::<Progress>(progress).unwrap().0 = 1.5;
    app.update();
    assert_ne!(
        before,
        app.world().get::<BackgroundColor>(toggle.entity).unwrap().0
    );
    assert_eq!(app.world().get::<Text>(toggle.label).unwrap().0, "RENAMED");
    let fill = app.world().get::<Children>(progress).unwrap()[0];
    assert_eq!(app.world().get::<Node>(fill).unwrap().width, percent(100));
    app.world_mut().get_mut::<Progress>(progress).unwrap().0 = f32::NAN;
    app.update();
    assert_eq!(app.world().get::<Node>(fill).unwrap().width, percent(0));
    app.world_mut().entity_mut(root).despawn();
    assert!(app.world().get_entity(toggle.entity).is_err());
    assert!(app.world().get_entity(fill).is_err());
}
