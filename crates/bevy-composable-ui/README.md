# bevy-composable-ui

A reusable workspace library for **Bevy 0.20**, used by two independent native
demos: [Shared Space](../../apps/composable-ui-prototype) and
[Field Notes](../../apps/garden-ui). It contains no demo resources, themes,
slot counts, layout breakpoints or fixed frame topology.

## Use it

```toml
[dependencies]
bevy-composable-ui = { path = "path/to/crates/bevy-composable-ui" }
bevy = { version = "0.20.0", default-features = false, features = ["2d", "ui", "default_font"] }
```

Add `ComposableUiPlugin` after your Bevy plugins for bindings and frame rendering.
`FrameRegionsPlugin` remains available for static frame-only applications.
Construct reusable functions
returning `impl View`, then mount their descriptions using `Commands`:

```rust,no_run
use bevy::prelude::*;
use bevy_composable_ui::prelude::*;

fn summary() -> impl View {
    column((
        text("A reusable view", 24., Color::WHITE),
        text("Content and frame styling are independent.", 12., Color::WHITE),
    )).gap(12.).pad(24.)
}

fn setup(mut commands: Commands) {
    let rail = Rail::new([
        Anchor::frame(0., 1.).offset(12., -12.),
        Anchor::frame(0., 0.).offset(12., 12.),
        Anchor::frame(1., 0.).offset(-12., 12.),
    ], 4., Color::srgb(0.4, 0.8, 0.7)).rounded(20.);

    let mut frame = column((summary(),)).pad(40.);
    frame.node.width = px(480);
    frame.node.height = px(240);
    frame.tagged(SharedFrame::new([rail])).mount(&mut commands);
}
```

`row`, `column`, and `grid` accept tuples (one through six children), vectors,
or `()` for no children. `Element::node` exposes Bevy's native layout properties
instead of duplicating its full style vocabulary. `padding` and `background`
on `View` wrap in declaration order; `pad`, `fill`, `border` and `gap` configure
an existing container. `tagged` initializes a typed Bevy bundle once per entity
lifetime, including app state or native headless controls. Its values are not
reinserted on reconciliation. Decorative views ignore picking;
an interactive wrapper must insert `Pickable::default()` as well as its behavior.

## Declarative state bindings

Bindings are typed lenses over ordinary Bevy resources. Existing application
systems can keep using `ResMut`; views update automatically.

```rust,no_run
use bevy::{prelude::*, ui_widgets::Button};
use bevy_composable_ui::prelude::*;

#[derive(Resource, Default)]
struct Model { count: u32, order: Vec<u64> }

fn setup(mut commands: Commands) {
    let count = Binding::resource(
        |model: &Model| model.count,
        |model: &mut Model, value| model.count = value,
    );
    let writer = count.clone();
    let increment = text("Increment", 16., Color::WHITE)
        .padding(12.)
        .tagged((Button, Pickable::default()))
        .on_activate(move |world| {
            writer.update(world, |value| *value += 1).unwrap();
        });
    let rows = Binding::read(|model: &Model| model.order.clone()).view(|ids| {
        column(ids.iter().map(|id| {
            text(format!("Row {id}"), 14., Color::WHITE).key(*id)
        }).collect::<Vec<_>>()).gap(8.)
    });
    column((count.view(|n| text(format!("Count: {n}"), 18., Color::WHITE)),
            increment, rows)).mount(&mut commands);
}
```

- `Binding::read` selects a read-only value. `Binding::resource` also provides
  a write-through setter. `get`, `set`, and `update` work with a `World`; equal
  writes do not change the resource. `map` creates a read-only projection.
- Selected values must be owned, `Clone + PartialEq + Send + Sync + 'static`.
  Select the fields a subtree needs. Selectors run each frame; rendering and
  reconciliation run when that selected value changes or a missing resource
  becomes available again.
- `.view` owns a stable transparent column host. Its child updates before Bevy
  UI preparation/layout in `PostUpdate`, after application updates and deferred
  actions. Nested bindings initialize immediately. `BoundView::node` and `.grow`
  configure the host's layout.
- Selector/render closures should be pure descriptions. `.on_activate` replaces
  the current callback without accumulating observers or retaining stale captures.
  It queues an action from Bevy's native `Activate` event. Applications supply
  native button behavior, tab navigation, and styles.
- Missing resources produce a `ViewDiagnostic` and retain the last valid body;
  an initially missing resource leaves an empty host. Reinsertion recovers.
  Removing the host drops its subscription and captured data.
- `ComposableUiPlugin` includes `ReactiveViewsPlugin` and `FrameRegionsPlugin`.
  Register either individually when only that capability is needed.

## Keyed reconciliation and focus

Use `.key(item.id)` on dynamic siblings. String and unsigned integer keys are
scoped to their parent. Reordering or inserting siblings preserves an entity
when its key and view kind match. Unkeyed children use structural position;
moving an item to a different parent starts a new lifetime.

Controlled properties (text, font size/color, layout, background and border)
update in place. Components initialized with `tagged`, including local state,
native hover/pressed behavior, scroll offsets and control tags, remain intact.
Tag bundle types and primitive kinds participate in compatibility; changing those
or a key creates a new entity. Padding/background wrappers are part of the
structural hierarchy. Custom views implement `View::into_node`; `mount` is supplied.

Bevy's `InputFocus` remains on the same entity through compatible updates and
reordering. Removing or replacing a focused node or ancestor clears focus rather
than transferring it to a different item. The demos draw a focus ring and provide
**R** to reverse items without moving focus to a reorder control. Clicking a
separate native control can move focus normally.

Duplicate sibling keys reject the affected update before entity changes. The
previous valid subtree remains with a `ViewDiagnostic`; a later valid selection
recovers. Removed items lose local state, and reinsertion starts a new lifetime.
Trees are limited to 64 description/binding levels. The library owns its mounted
child hierarchies; update models or use `reconcile(world, root, view)` instead of
editing those relationships directly. That function returns a potentially new
root ID when its key or kind changes.

## Anchored frame regions

Attach `SharedFrame` to a normal UI container. Each `Rail` is an orthogonal
polyline with logical-pixel width, paint and optional circular bend radius.
Anchors resolve after Bevy layout:

- `AxisAnchor::Frame(fraction)` uses the owning frame's border box.
- `AxisAnchor::Node(entity, fraction)` uses a resolved node's border box.
- `AxisAnchor::Between((a, fraction_a), (b, fraction_b))` uses the midpoint
  between two resolved edges. Use 1 and 0 for the corridor between adjacent views.
- `Anchor::at(x, y)` combines independent coordinates; `.offset(x, y)` adds dp.

The plugin converts transformed node bounds into frame-local coordinates,
accounts for DPI, resolves all routes, and updates an embedded UI material.
Anchored views must share a UI render target and coordinate space with the
frame. The owning node must have nonzero layout allocation. Reserve clearances
using normal layout padding/gaps; frame paint does not allocate space itself.
Do not attach another `MaterialNode` to the same owner. Multiple independent
owners work in the same application; the garden demo uses two.
Frame owners remain decorative and pass-through; put interactive controls in
their child views rather than making the frame owner itself a button.

All rails in an owner have one coverage evaluation, so transparent junctions
do not darken twice. Where paints overlap, the closest signed boundary chooses
the paint; exact ties choose the earlier rail. Shared opacity multiplies that
paint's alpha. This is a paint partition, not source-over layering of each rail.
Different owners composite independently, so put a shared junction in one owner.

## Geometry contract and errors

`SharedFrame::resolve(size, node_rect)` exposes the same geometry resolution used
by the plugin without requiring a GPU. Its `FrameGeometry` supports segment
count and unclipped geometric `contains` queries. This does not install shaped
picking; owner paint remains pass-through. Rendering is bounded by the owning
UI quad and ancestor clips.

- Routes support axis-aligned runs, all four circular bend orientations, and
  round caps/joins. Redundant adjacent and collinear points are removed.
- Each bend radius is clamped to half of each adjacent run, keeping neighboring
  bends disjoint. A positive radius below half the rail width is rejected.
  Radius zero uses straight capsule segments with round joins.
- Width zero has no coverage. Empty frame collections are valid.
- Non-finite inputs, negative sizes/widths/radii, invalid opacity, diagonal or
  reversing routes, missing nodes and impossible bends return `FrameError`.
- There are **32 line/arc segments per frame owner**. Each rounded bend adds an
  arc plus its adjoining line segments. Exceeding the budget fails atomically.
- `FrameDiagnostic` reports the last failure. Invalid edits retain the last valid
  geometry; an initially invalid owner is transparent. Resolution retries, so
  an anchor becoming available can recover. Unchanged uniform data is not uploaded.
- Removing `SharedFrame` removes the renderer-owned material/diagnostic on the
  next update. The plugin embeds its shader; callers need no asset files.

Applications own controls, themes, responsive decisions, scrolling and resource models.
Bevy remains the sole layout engine. This library does not yet include
arbitrary path Boolean operations, shaped descendant clipping,
or platform-specific text editing/accessibility integration.

## Validation

```sh
cargo test -p bevy-composable-ui
cargo clippy -p bevy-composable-ui -p composable-ui-prototype -p garden-ui --all-targets -- -D warnings
cargo run -p composable-ui-prototype -- --capture-dir output/ui-library/original
cargo run -p garden-ui -- --capture-dir output/ui-library/garden
```

The public-interface tests exercise composition, binding reads/writes, selective
updates, keyed reordering/insertion/removal, focus and local-state retention,
callback replacement, rejected-update recovery and bounded recursive bindings.
Geometry tests cover anchor resolution, four curve orientations, resize behavior
and failures. Both rendered replays verify retained entity identities, focused
controls, bound text, sibling order, responsive layouts and clear diagnostics.
Replay captures render to an offscreen GPU target; normal launches use the window.
