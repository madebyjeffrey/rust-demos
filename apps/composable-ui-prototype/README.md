# Composable UI prototype

A small, in-memory native Bevy executable exploring the composition and shared
frame ideas from the [design specification](../../output/pdf/composable-rust-bevy-ui-specification.pdf).

```sh
cargo run -p composable-ui-prototype
```

## Try it

- **1 / 2 / 3**, or the profile buttons: thin (1.5/1.5 dp), mixed (24/6 dp),
  and thick (24/24 dp) shell/divider widths.
- Resize below **820 logical pixels**: the displays stack vertically. Enlarge
  the window to return to the wide arrangement. **S / Stack** forces the stacked plan.
- Compact panels reduce spacing and hide secondary diagnostic rows. In short
  windows, use the wheel/trackpad or **Page Up / Page Down / Home / End** to scroll.
  Keyboard focus brings off-screen buttons into view.
- **G / Guides**: show dashed rail centerlines.
- **A / 50% alpha**: inspect translucent junctions. Both rails are evaluated
  and composited together, so their intersection does not accumulate opacity.
- **Record sample** updates the chart and counter. **Space** also records a
  sample when no button has keyboard focus. **Tab**, then **Enter / Space**,
  uses Bevy's headless button focus/activation behavior.

The three display entities remain mounted across profile and layout changes.
Their state lives in a Bevy resource; sample counts survive resizing.
The footer exposes the active layout, widths, profile and count.

## Reactive identity and focus

Sensor chips in Telemetry are a resource-bound keyed collection. Activate a chip
to increment its value through a writable `Binding`. The amber ring marks focus.

- **R / Reverse** reverses the collection while retaining chip entities.
- **N / Add** appends a new keyed sensor (up to five).
- **D / Remove** removes the last sensor; focus clears if it was focused.

Use the shortcuts while a chip has focus to see it stay on the same sensor
through reorder and insertion. Clicking a separate button moves focus normally.
Counters, chart bars, chip labels and status text use `Binding::view`; there is
no manual text or shape refresh system.

## Composition and frame ownership

[bevy-composable-ui](../../crates/bevy-composable-ui) provides the `View` trait,
tuple/vector child composition, `row`, `column`, text, ordered padding/background
wrappers, and typed ECS tags. This app imports that library directly.
Application components such as `telemetry()`, `diagnostics()` and `session()`
return `impl View` and combine these values. Ordinary panels use normal Bevy
borders. Controls use Bevy 0.20 headless buttons with custom styling.

`configure_frame` in `src/main.rs` declares two routes using the library's
`SharedFrame`, `Rail`, `Anchor` and `AxisAnchor` values. The library resolves
actual post-layout rectangles and corridor midpoints, including a different
route when the displays stack. Fixed 56 dp
top/left reservations and 48/52 dp inter-display corridors accommodate every
profile without changing the content to make room for thicker paint.

The library embeds its generic frame shader. One owner paints the union of an
outer curved spine and an internal curved divider, with independent widths and
paints. Their intersection has one coverage evaluation; distinct paints use a
signed-distance partition. Curves are circular centerline arcs with round caps.
Physical pixels are converted to logical coordinates for DPI-aware thickness.
Decorative entities ignore picking, leaving the controls interactive.

## Reproduce the visual review

```sh
cargo run -p composable-ui-prototype -- --capture-dir output/ui-prototype
```

This opens a real rendered window, activates the same typed button events,
captures thin and translucent thick frames, resizes to a compact mixed layout,
records three samples, checks a short scrolled viewport, restores the wide layout,
then exits. Five PNGs and
state/size reports are written to the directory. The replay asserts that
display entity IDs and sample state survive reflow. It requires a graphical
session and a working graphics adapter. Capture mode renders the same camera/UI
to an offscreen image so native surface availability does not affect captures;
interactive runs use the normal window target. It also checks keyed sensor IDs,
child order, focused entity, bound text, and clear binding diagnostics.

## Prototype boundary

This is a native integration sketch, not the full framework described in the
PDF. Bevy's existing Flexbox/text layout is the sole layout authority; there is
no separate Taffy runtime, custom layout protocol, general
path Boolean engine, shaped descendant clipping or shaped-region picking.
This app supplies the shell/divider topology; the shared library supports other
orthogonal routes, as demonstrated by `garden-ui`. Native iOS packaging,
safe-area/IME integration and mobile performance/accessibility validation are
outside this desktop prototype. No benchmark or full specification conformance
is claimed.
