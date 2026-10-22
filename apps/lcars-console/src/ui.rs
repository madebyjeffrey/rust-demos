//! The science-console screen, expressed as content and composition.
use super::{Console, Control, ScanMode, Sector};
use bevy::{picking::Pickable, prelude::*};
use lcars_ui::{Screen, TextStyle, Tone, Ui};

#[derive(Component)]
pub(super) struct Readout(pub ReadoutKind);
pub(super) enum ReadoutKind {
    Status,
    Target,
    Progress,
    Channels,
    Results,
    ScanLabel,
}
#[derive(Component)]
pub(super) struct SignalBar(pub usize);

pub(super) fn setup(mut commands: Commands, state: Res<Console>) {
    commands.spawn(Camera2d);
    let screen = Screen::new("LCARS // SCIENCE OPERATIONS", "Long-range sensor array")
        .subtitle("SCIENCE DIVISION / DECK 04 / NCC 1701-D")
        .identifier("47-082")
        .rail(["01-284", "02-947", "03-186", "04-730"])
        .spawn(&mut commands, |ui| {
            ui.column(3.0, |ui| targeting(ui, &state));
            ui.column(4.0, telemetry);
            ui.column(2.4, |ui| channels(ui, &state));
        });
    commands
        .entity(screen.status)
        .insert(Readout(ReadoutKind::Status));
}

fn targeting(ui: &mut Ui, state: &Console) {
    ui.section("01 / TARGET SECTOR", Tone::Orange, |ui| {
        ui.text("SELECT ONE", TextStyle::Caption);
        ui.radio_group(
            Tone::Orange,
            Control::Sector(state.sector),
            [
                (Control::Sector(Sector::Alpha), "ALPHA QUADRANT"),
                (Control::Sector(Sector::Beta), "BETA QUADRANT"),
                (Control::Sector(Sector::Gamma), "GAMMA QUADRANT"),
            ],
        );
    });
    ui.spacer();
    ui.section("02 / SCAN PROFILE", Tone::Lilac, |ui| {
        ui.text("SELECT ONE / ARROW KEYS TO CYCLE", TextStyle::Caption);
        ui.radio_group(
            Tone::Lilac,
            Control::Mode(state.mode),
            [
                (Control::Mode(ScanMode::Passive), "PASSIVE / 04 SEC"),
                (Control::Mode(ScanMode::Active), "ACTIVE / 07 SEC"),
                (Control::Mode(ScanMode::Deep), "DEEP / 10 SEC"),
            ],
        );
    });
}

fn telemetry(ui: &mut Ui) {
    ui.section("03 / SENSOR TELEMETRY", Tone::Blue, |ui| {
        ui.readout("", TextStyle::Muted, Readout(ReadoutKind::Target));
        spectrum(ui);
        ui.readout("", TextStyle::Value, Readout(ReadoutKind::Progress));
        ui.progress(0.0, Tone::Orange, ());
        ui.readout("", TextStyle::Muted, Readout(ReadoutKind::Results));
    });
    ui.spacer();
    ui.row(|ui| {
        let scan = ui.primary_button("INITIATE SCAN", Tone::Orange, Control::Scan);
        ui.bind(scan.label, Readout(ReadoutKind::ScanLabel));
        ui.button("HOLD", Tone::Lilac, Control::Hold);
    });
}

fn channels(ui: &mut Ui, state: &Console) {
    ui.section("04 / CHANNELS", Tone::Blue, |ui| {
        ui.text("SELECT MULTIPLE", TextStyle::Caption);
        ui.stack(|ui| {
            for (index, label) in ["SUBSPACE", "GRAVIMETRIC", "ELECTROMAG."]
                .into_iter()
                .enumerate()
            {
                ui.toggle(
                    label,
                    Tone::Blue,
                    state.channels[index],
                    Control::Channel(index),
                );
            }
        });
        ui.readout("", TextStyle::Muted, Readout(ReadoutKind::Channels));
    });
    ui.separator(Tone::Lilac);
    ui.text("ARRAY STATUS", TextStyle::Muted);
    ui.text(
        "UPLINK / STABLE\nRANGE  / 14.2 LY\nSIGNAL / NOMINAL",
        TextStyle::Body,
    );
    ui.spacer();
    ui.button("RESET CONSOLE", Tone::Peach, Control::Reset);
}

// Custom plots can use ordinary Bevy UI inside a toolkit-managed layout region.
fn spectrum(ui: &mut Ui) {
    ui.custom(
        Node {
            width: percent(100),
            height: px(230),
            flex_shrink: 0.0,
            ..default()
        },
        |p| {
            for row in 0..5 {
                p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(4),
                        top: px(24.0 + row as f32 * 43.0),
                        width: percent(92),
                        height: px(1),
                        ..default()
                    },
                    BackgroundColor(lcars_ui::INK),
                    Pickable::IGNORE,
                ));
            }
            for index in 0..30 {
                p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(4.5 + index as f32 * 3.05),
                        top: px(55),
                        width: percent(1.5),
                        height: px(150),
                        ..default()
                    },
                    BackgroundColor(if index % 5 == 0 {
                        Tone::Orange.color()
                    } else {
                        Tone::Blue.color()
                    }),
                    SignalBar(index),
                    Pickable::IGNORE,
                ));
            }
            for (label, y) in [
                ("SPECTRAL RETURN / SIMULATED", 9.0),
                ("0.02                 4.70 THZ", 211.0),
            ] {
                p.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Tone::Blue.color()),
                    Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(4),
                        top: px(y),
                        ..default()
                    },
                ));
            }
        },
    );
}
