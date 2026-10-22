# lcars-ui

A small composition interface for LCARS-inspired Bevy 0.20 screens. It turns a
screen into a hierarchy of named sections and controls while keeping application
state in ordinary Bevy resources. The science-console demo uses this crate.

```rust,no_run
use bevy::prelude::*;
use lcars_ui::{LcarsPlugin, Screen, TextStyle, Tone};

#[derive(Component, PartialEq)]
enum Action { Passive, Active, Subspace, Scan }

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, LcarsPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    Screen::new("LCARS // SCIENCE", "Sensor control")
        .subtitle("SCIENCE DIVISION / DECK 04")
        .spawn(&mut commands, |ui| {
            ui.column(1.0, |ui| {
                ui.section("SCAN PROFILE", Tone::Lilac, |ui| {
                    ui.text("SELECT ONE", TextStyle::Caption);
                    ui.radio_group(Tone::Lilac, Action::Passive, [
                        (Action::Passive, "PASSIVE SCAN"),
                        (Action::Active, "ACTIVE SCAN"),
                    ]);
                });
            });
            ui.column(1.0, |ui| {
                ui.section("SENSOR CHANNELS", Tone::Blue, |ui| {
                    ui.toggle("SUBSPACE", Tone::Blue, true, Action::Subspace);
                });
                ui.spacer();
                ui.primary_button("INITIATE SCAN", Tone::Orange, Action::Scan);
            });
        });
}
```

This snippet constructs the screen. To make selections persistent, handle the
native widget events and synchronize `Checked` as described below. A complete,
interactive example including those systems is available with:

```sh
cargo run -p lcars-ui --example engineering
```

## Composition

| Method | Responsibility |
| --- | --- |
| `Screen::new(title, heading)` | Curved frame, header, rail, status area, footer, and window fitting |
| `column(weight, contents)` | A proportional column within a row |
| `row(contents)` / `stack(contents)` | Horizontal / vertical flow with shared spacing |
| `section(title, tone, contents)` | Heading followed by related content |
| `spacer()` | Absorbs remaining space; useful for bottom-aligned actions |
| `radio_group(tone, selected, options)` | Native radio group, Tab focus and arrow-key navigation |
| `toggle(label, tone, checked, binding)` | Independent native checkbox |
| `button(...)` / `primary_button(...)` | Outlined / filled action button |
| `text(...)` / `readout(..., binding)` | Typography roles and dynamic text bindings |
| `progress(value, tone, binding)` | Progress display driven by a `Progress` component |
| `separator(tone)` | A decorative horizontal bar |
| `custom(node, children)` / `spawn(bundle)` | Ordinary Bevy UI for custom displays |

The screen's content region is a row. Its columns divide the available width by
weight. Inside a column, content flows vertically. A radio group owns its option
spacing and keyboard focus. No screen coordinates or manual corner insets appear
in the application composition.

## State and events

Widgets are **controlled**: the application is the source of truth. This avoids a
second hidden state store and makes reset, loading saved state, and external
updates behave the same way as clicks.

1. Pass an application component or bundle as the widget binding. For radio
   groups, each option value is itself a component and implements `PartialEq`.
2. Observe Bevy `Activate` for buttons or `ValueChange<bool>` for radios/toggles.
   Query your binding on `event.entity` or `event.source` to identify the action.
3. Change your own resource, then synchronize `Checked` on the relevant widgets
   in `Update`. For radios, exactly one option in each group must be checked.
4. The toolkit presents that state in `PostUpdate`, including the selection mark,
   fill color, hover, press, keyboard focus, and `InteractionDisabled` styling.

Initial radio selection must match exactly one option; invalid construction
panics before creating the group. Column weights must be positive and finite.

Buttons and toggles return `Widget { entity, label }`. Use `ui.bind(widget.label,
YourReadout)` to query and update the native `Text` without traversing children.
The toolkit never replaces that label. `readout` attaches a binding directly to a
text entity. `Screen::spawn` returns the root and status-text entities.

Progress is a `Progress(f32)` component on the returned track entity. Values are
clamped for display; non-finite values display zero. Despawning the returned
screen root removes all of its descendants.

## What belongs where

- `frame.rs`: frame geometry and fitting. The inside corner tangents and bar edges
  share dimensions, so callers cannot accidentally recreate the ledge defect.
- `builder.rs`: layout composition, typography, native widget construction.
- `presentation.rs`: common interaction appearance and progress rendering.
- The application: domain state, event handlers, validation, and custom telemetry.

The screen builder runs once during setup; systems update existing entities.
There is no custom event bus, string-based action dispatch, per-frame rebuild,
or separate layout language to maintain.

## Current scope

This first version has a standard four-segment left rail and a 1280×800 desktop
canvas fitted to the primary window. Fitting is local to the screen and does not
change Bevy's global `UiScale`. It is proportional scaling, not mobile reflow.
Content must fit the canvas; there is no automatic scrolling or pagination.
Keep header and rail labels short. The palette currently has four fixed tones.

Install `LcarsPlugin` once alongside `DefaultPlugins` with Bevy's `ui` feature;
add a UI camera yourself. The toolkit supplies tab navigation if it is not already
installed. Callers retain access to native Bevy entities, components, observers,
and custom rendering through the public escape hatches.

Additional frame arrangements, themes, and responsive reflow can be introduced
behind this composition interface when a real screen needs them.
