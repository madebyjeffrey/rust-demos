//! A second screen built entirely through the public LCARS interface.
use bevy::{
    prelude::*,
    ui::Checked,
    ui_widgets::{Activate, ValueChange},
};
use lcars_ui::{LcarsPlugin, Screen, TextStyle, Tone};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Control {
    Cruise,
    Auxiliary,
    Shields,
    Reset,
}
#[derive(Resource)]
struct Engineering {
    mode: Control,
    shields: bool,
}
impl Default for Engineering {
    fn default() -> Self {
        Self {
            mode: Control::Cruise,
            shields: true,
        }
    }
}
fn main() {
    App::new()
        .add_plugins((DefaultPlugins, LcarsPlugin))
        .init_resource::<Engineering>()
        .add_systems(Startup, setup)
        .add_observer(select)
        .add_observer(reset)
        .add_systems(Update, synchronize)
        .run();
}
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    Screen::new("LCARS // ENGINEERING", "Power distribution")
        .subtitle("ENGINEERING DIVISION / DECK 12")
        .rail(["CORE", "GRID", "AUX", "SYS"])
        .spawn(&mut commands, |ui| {
            ui.column(1.0, |ui| {
                ui.section("01 / POWER SOURCE", Tone::Orange, |ui| {
                    ui.text("SELECT ONE / ARROW KEYS TO CYCLE", TextStyle::Caption);
                    ui.radio_group(
                        Tone::Orange,
                        Control::Cruise,
                        [
                            (Control::Cruise, "MAIN REACTOR"),
                            (Control::Auxiliary, "AUXILIARY POWER"),
                        ],
                    );
                });
                ui.spacer();
                ui.button("RESTORE DEFAULTS", Tone::Peach, Control::Reset);
            });
            ui.column(1.0, |ui| {
                ui.section("02 / DEFENSIVE SYSTEMS", Tone::Blue, |ui| {
                    ui.toggle("SHIELDS ONLINE", Tone::Blue, true, Control::Shields);
                    ui.text("Reserve energy", TextStyle::Body);
                    ui.progress(0.72, Tone::Blue, ());
                    ui.text("SIMULATED SYSTEM / 72% RESERVE", TextStyle::Muted);
                });
            });
        });
}
fn select(event: On<ValueChange<bool>>, controls: Query<&Control>, mut state: ResMut<Engineering>) {
    match controls.get(event.source) {
        Ok(Control::Shields) => state.shields = event.value,
        Ok(mode @ (Control::Cruise | Control::Auxiliary)) if event.value => state.mode = *mode,
        _ => {}
    }
}
fn reset(event: On<Activate>, controls: Query<&Control>, mut state: ResMut<Engineering>) {
    if controls.get(event.entity) == Ok(&Control::Reset) {
        *state = Engineering::default();
    }
}
fn synchronize(
    mut commands: Commands,
    state: Res<Engineering>,
    controls: Query<(Entity, &Control)>,
) {
    if !state.is_changed() {
        return;
    }
    for (entity, control) in &controls {
        let checked = match control {
            Control::Shields => state.shields,
            Control::Cruise | Control::Auxiliary => *control == state.mode,
            Control::Reset => continue,
        };
        if checked {
            commands.entity(entity).insert(Checked);
        } else {
            commands.entity(entity).remove::<Checked>();
        }
    }
}
