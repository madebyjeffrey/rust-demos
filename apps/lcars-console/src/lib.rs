//! Application state and behavior for the LCARS science-console demo.
//! Screen composition lives in `ui.rs`; reusable UI behavior lives in `lcars-ui`.
mod ui;
use bevy::{
    prelude::*,
    ui::Checked,
    ui_widgets::{Activate, ValueChange},
};
use lcars_ui::{INK, LcarsPlugin, Progress};
use ui::{Readout, ReadoutKind, SignalBar};

pub struct LcarsConsolePlugin;
impl Plugin for LcarsConsolePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(INK))
            .init_resource::<Console>()
            .add_plugins(LcarsPlugin)
            .add_observer(on_action)
            .add_observer(on_selection)
            .add_systems(Startup, ui::setup)
            .add_systems(Update, (tick_scan, sync_controls, update_readouts).chain());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sector {
    Alpha,
    Beta,
    Gamma,
}
impl Sector {
    fn label(self) -> &'static str {
        match self {
            Self::Alpha => "ALPHA",
            Self::Beta => "BETA",
            Self::Gamma => "GAMMA",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanMode {
    Passive,
    Active,
    Deep,
}
impl ScanMode {
    fn label(self) -> &'static str {
        match self {
            Self::Passive => "PASSIVE",
            Self::Active => "ACTIVE",
            Self::Deep => "DEEP",
        }
    }
    fn duration(self) -> f32 {
        match self {
            Self::Passive => 4.0,
            Self::Active => 7.0,
            Self::Deep => 10.0,
        }
    }
}
#[derive(Resource)]
struct Console {
    sector: Sector,
    mode: ScanMode,
    channels: [bool; 3],
    progress: f32,
    scanning: bool,
    completed: u32,
    status: String,
}
impl Default for Console {
    fn default() -> Self {
        Self {
            sector: Sector::Alpha,
            mode: ScanMode::Passive,
            channels: [true, true, false],
            progress: 0.0,
            scanning: false,
            completed: 0,
            status: "SYSTEM READY / Awaiting scan authorization".into(),
        }
    }
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Control {
    Sector(Sector),
    Mode(ScanMode),
    Channel(usize),
    Scan,
    Hold,
    Reset,
}
impl Control {
    fn selected(self, state: &Console) -> bool {
        match self {
            Self::Sector(v) => state.sector == v,
            Self::Mode(v) => state.mode == v,
            Self::Channel(i) => state.channels[i],
            _ => false,
        }
    }
}
fn on_selection(
    event: On<ValueChange<bool>>,
    controls: Query<&Control>,
    mut state: ResMut<Console>,
) {
    let Ok(control) = controls.get(event.source) else {
        return;
    };
    match *control {
        Control::Sector(sector) if event.value => {
            state.sector = sector;
            state.progress = 0.0;
            state.scanning = false;
            state.status = format!("TARGET ACQUIRED / {} quadrant selected", sector.label());
        }
        Control::Mode(mode) if event.value => {
            state.mode = mode;
            state.progress = 0.0;
            state.scanning = false;
            state.status = format!("PROFILE UPDATED / {} sensors standing by", mode.label());
        }
        Control::Channel(index) => {
            state.channels[index] = event.value;
            state.progress = 0.0;
            state.scanning = false;
            state.status = "CHANNEL CONFIGURATION UPDATED / Scan reset".into();
        }
        _ => {}
    }
}

fn on_action(event: On<Activate>, controls: Query<&Control>, mut state: ResMut<Console>) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    match control {
        Control::Scan => {
            if !state.channels.iter().any(|enabled| *enabled) {
                state.status = "SCAN INHIBITED / Select at least one sensor channel".into();
            } else {
                if state.progress >= 1.0 {
                    state.progress = 0.0;
                }
                state.scanning = true;
                state.status = format!(
                    "SCAN IN PROGRESS / {} quadrant / {} profile",
                    state.sector.label(),
                    state.mode.label()
                );
            }
        }
        Control::Hold => {
            state.scanning = false;
            state.status = "SCAN ON HOLD / Initiate scan to resume".into();
        }
        Control::Reset => *state = Console::default(),
        _ => {}
    }
}

fn tick_scan(time: Res<Time>, mut state: ResMut<Console>) {
    if state.scanning {
        state.progress = (state.progress + time.delta_secs() / state.mode.duration()).min(1.0);
        if state.progress >= 1.0 {
            state.scanning = false;
            state.completed += 1;
            state.status = format!(
                "SCAN COMPLETE / {} quadrant survey archived",
                state.sector.label()
            );
        }
    }
}

// Application state is authoritative; the toolkit presents native Checked state.
fn sync_controls(
    mut commands: Commands,
    state: Res<Console>,
    controls: Query<(Entity, &Control, Has<Checked>)>,
) {
    if !state.is_changed() {
        return;
    }
    for (entity, control, checked) in &controls {
        let selected = control.selected(&state);
        if selected != checked {
            if selected {
                commands.entity(entity).insert(Checked);
            } else {
                commands.entity(entity).remove::<Checked>();
            }
        }
    }
}

fn update_readouts(
    state: Res<Console>,
    time: Res<Time>,
    mut texts: Query<(&Readout, &mut Text)>,
    mut progress: Query<&mut Progress>,
    mut bars: Query<(&SignalBar, &mut Node)>,
) {
    if state.is_changed() {
        for (readout, mut text) in &mut texts {
            text.0 = match readout.0 {
                ReadoutKind::Status => state.status.clone(),
                ReadoutKind::Target => format!(
                    "TARGET: {} / PROFILE: {}",
                    state.sector.label(),
                    state.mode.label()
                ),
                ReadoutKind::Progress => format!(
                    "{:03.0}% / {}",
                    state.progress * 100.0,
                    if state.scanning {
                        "ACQUIRING"
                    } else if state.progress >= 1.0 {
                        "COMPLETE"
                    } else {
                        "STANDBY"
                    }
                ),
                ReadoutKind::Channels => format!(
                    "{} / 3 CHANNELS ONLINE",
                    state.channels.iter().filter(|v| **v).count()
                ),
                ReadoutKind::Results => format!("SURVEYS ARCHIVED / {:03}", state.completed),
                ReadoutKind::ScanLabel => if state.scanning {
                    "SCANNING..."
                } else if state.progress > 0.0 && state.progress < 1.0 {
                    "RESUME SCAN"
                } else {
                    "INITIATE SCAN"
                }
                .into(),
            };
        }
        for mut progress in &mut progress {
            progress.0 = state.progress;
        }
    }
    let phase = state.progress * 18.0;
    let idle = if state.scanning {
        time.elapsed_secs() * 2.0
    } else {
        0.0
    };
    for (SignalBar(index), mut node) in &mut bars {
        let x = *index as f32;
        let height = 20.0 + ((x * 0.61 + phase).sin() * (x * 0.23 + idle).cos()).abs() * 142.0;
        node.height = px(height);
        node.top = px(199.0 - height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ui_widgets::{Checkbox, RadioButton};
    use std::time::Duration;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Console>()
            .init_resource::<Time>()
            .add_observer(on_action)
            .add_observer(on_selection)
            .add_systems(Update, tick_scan);
        app
    }
    fn select(app: &mut App, control: Control, value: bool) {
        let entity = app.world_mut().spawn(control).id();
        app.world_mut().trigger(ValueChange {
            source: entity,
            value,
            is_final: true,
        });
    }
    fn activate(app: &mut App, control: Control) {
        let entity = app.world_mut().spawn(control).id();
        app.world_mut().trigger(Activate { entity });
    }
    #[test]
    fn radio_groups_are_exclusive_and_independent() {
        let mut app = app();
        select(&mut app, Control::Sector(Sector::Beta), true);
        select(&mut app, Control::Sector(Sector::Gamma), true);
        select(&mut app, Control::Mode(ScanMode::Deep), true);
        let state = app.world().resource::<Console>();
        assert_eq!(state.sector, Sector::Gamma);
        assert_eq!(state.mode, ScanMode::Deep);
        assert!(!Control::Sector(Sector::Beta).selected(state));
        assert!(Control::Sector(Sector::Gamma).selected(state));
    }
    #[test]
    fn channels_are_independent_and_empty_selection_blocks_scan() {
        let mut app = app();
        select(&mut app, Control::Channel(2), true);
        assert_eq!(app.world().resource::<Console>().channels, [true; 3]);
        for i in 0..3 {
            select(&mut app, Control::Channel(i), false);
        }
        activate(&mut app, Control::Scan);
        let state = app.world().resource::<Console>();
        assert!(!state.scanning);
        assert!(state.status.contains("INHIBITED"));
    }
    #[test]
    fn scan_holds_resumes_completes_once_and_resets() {
        let mut app = app();
        activate(&mut app, Control::Scan);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(2));
        app.update();
        assert_eq!(app.world().resource::<Console>().progress, 0.5);
        activate(&mut app, Control::Hold);
        app.update();
        assert_eq!(app.world().resource::<Console>().progress, 0.5);
        activate(&mut app, Control::Scan);
        app.update();
        app.update();
        let state = app.world().resource::<Console>();
        assert_eq!(state.progress, 1.0);
        assert_eq!(state.completed, 1);
        assert!(!state.scanning);
        activate(&mut app, Control::Reset);
        let state = app.world().resource::<Console>();
        assert_eq!(state.completed, 0);
        assert_eq!(state.progress, 0.0);
        assert_eq!(state.channels, [true, true, false]);
    }
    #[test]
    fn changing_configuration_cancels_partial_scan() {
        let mut app = app();
        activate(&mut app, Control::Scan);
        app.world_mut().resource_mut::<Console>().progress = 0.5;
        select(&mut app, Control::Mode(ScanMode::Active), true);
        let state = app.world().resource::<Console>();
        assert!(!state.scanning);
        assert_eq!(state.progress, 0.0);
    }

    #[test]
    fn scene_keeps_widget_selection_and_readouts_in_sync() {
        let mut app = app();
        app.add_plugins(LcarsPlugin)
            .add_systems(Startup, ui::setup)
            .add_systems(Update, (sync_controls, update_readouts).chain());
        app.update();
        select(&mut app, Control::Sector(Sector::Beta), true);
        select(&mut app, Control::Channel(2), true);
        app.update();
        let world = app.world_mut();
        let mut controls =
            world.query::<(&Control, Has<Checked>, Has<RadioButton>, Has<Checkbox>)>();
        let mut selected_radios = 0;
        let mut selected_channels = 0;
        for (control, checked, radio, checkbox) in controls.iter(world) {
            if radio && checked {
                selected_radios += 1;
            }
            if checkbox && checked {
                selected_channels += 1;
            }
            if radio && matches!(control, Control::Sector(Sector::Alpha)) {
                assert!(!checked);
            }
            if radio && matches!(control, Control::Sector(Sector::Beta)) {
                assert!(checked);
            }
        }
        assert_eq!(selected_radios, 2);
        assert_eq!(selected_channels, 3);
        let mut readouts = world.query::<(&Readout, &Text)>();
        assert!(
            readouts
                .iter(world)
                .any(|(readout, text)| matches!(readout.0, ReadoutKind::Target)
                    && text.0.contains("BETA"))
        );
        activate(&mut app, Control::Reset);
        app.update();
        let world = app.world_mut();
        assert_eq!(
            controls
                .iter(world)
                .filter(|(_, checked, _, checkbox)| *checked && *checkbox)
                .count(),
            2
        );
    }
}
