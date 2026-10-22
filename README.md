# rust-demos

Independent Bevy demos live in the workspace under `apps/`.

## Reusable LCARS UI

The workspace includes [`lcars-ui`](crates/lcars-ui/README.md), a reusable Bevy UI
crate. It provides a framed screen, proportional columns, sections, buttons,
radio groups, toggles, readouts, and progress bars. See the
[science-console composition](apps/lcars-console/src/ui.rs) for the main example.

A smaller, independent engineering screen demonstrates the same interface:

```sh
cargo run -p lcars-ui --example engineering
```

## LCARS Console

Launch the LCARS-inspired science console with:

```sh
cargo run -p lcars-console
```

The console uses Bevy's native UI, with a curved segmented frame, orange and
lavender controls, and an animated simulated sensor display. Choose a target
sector and scan profile with single-choice radio groups, then independently
enable any combination of the three sensor channels. Initiate Scan starts a
timed survey; Hold pauses it, Resume Scan continues it, and Reset Console restores
the defaults. Changing the configuration resets the current scan. At least one
channel must be enabled before scanning. Completed surveys increment the archive
counter. Telemetry is illustrative and does not connect to external services.

Click controls, or use Tab / Shift+Tab to move focus. Use arrow keys (or Home / End)
within a radio group, and Enter / Space to activate buttons and channel toggles.
Hover, press, focus, and selection have visible feedback. The console scales to
fit the desktop window, with a minimum size of 960×600 logical points.

Run its interaction tests with `cargo test -p lcars-console`.

## Galaxy Menu

Launch the first demo with:

```sh
cargo run -p galaxy-menu
```

The menu provides New Game, Load Game, and Settings buttons with placeholder actions.
Desktop also provides Quit, which exits the application; iPhone and iPad omit it.
Click or tap a button to activate it. On desktop, use Tab and Shift+Tab to move
keyboard focus, then Enter or Space to activate the focused button. Hover, press,
and keyboard focus have visible feedback. Drags beginning on a button belong to
the menu until release or cancellation; unused space around the controls remains
available for exploring the galaxy backdrop.

The galaxy backdrop fills the window behind the menu, with stars, gas, dust and
orbiting companions. Drag with the left mouse button to orbit continuously through
the poles and use the mouse wheel to zoom within the scene's framing bounds.
Menu overlays share the full window without reserving a separate scene viewport.
Controls reflow from the available logical window size and native safe areas,
including rotation and iPad multitasking. Compact windows use fewer decorative
labels or a grid while retaining readable buttons. Resizing preserves orbit
and zoom relative to the galaxy's full-window fit. Desktop resizing has a minimum
of 240×200 logical points so every action remains operable with readable labels
and at least 44-point targets.

Build and directly install the same application on registered iPhone and iPad devices
with the [native iOS workflow](docs/ios.md).
