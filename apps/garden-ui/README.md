# Field Notes / garden UI

```sh
cargo run -p garden-ui
```

A second consumer of [bevy-composable-ui](../../crates/bevy-composable-ui).
It uses warm paper colors, forest/clay rails, a sidebar, and a CSS Grid mosaic:
the growth card spans two rows beside watering and journal cards. Below 900 dp,
the sidebar disappears and the same cards stack in one column. Secondary growth
meters hide while the leaf count remains visible. Short windows scroll.

The shared main frame turns along the **bottom and right**, with an independently
anchored internal separator. The sidebar has its own small frame owner. Both
use the same library renderer as the original dark telemetry demo; there is no
garden-specific shader or copied composition implementation.

- **T / Fine lines** toggles the main bracket between 10 and 1.5 dp.
- **G / Guides** displays the main frame centerlines.
- **Water plants** increments an in-memory, resource-bound watering counter.
- Journal plant chips use writable bindings. Activate one to increment its own
  visit count; a clay ring marks focus.
- **R / Reverse**, **N / Add** (up to five), and **D / Remove** edit the keyed
  plant collection. Use shortcuts while a chip has focus: reorder/insertion
  preserve focus, and removal of the focused chip clears it.
- **Space** waters when no button has focus; **Tab / Enter / Space** activate
  focused native buttons.
- Wheel/trackpad, **Page Up / Page Down / Home / End** scroll short windows.

Plant chips use 520 ms horizontal slide/scale transitions for insertion/removal
and glide to their new slots on reorder. Their entity IDs and focus outlines
travel with the plants. Rapid edits retarget from the current pose. Removed
plants immediately stop accepting input, then finish their visual exit.

```sh
cargo run -p garden-ui -- --capture-dir output/ui-transitions/garden
```

The replay captures wide, fine-line, compact and restored layouts. It activates
native button events, preserves watering state and plot entities across resize,
and checks diagnostics for both independent frame owners. Four PNGs and state
reports are saved before exit. A graphical session and graphics device are required.

Counters, collection text and status are declared with `Binding::view`; no manual
text refresh system is used. The replay checks focused keyed identities, child
order, automatic text updates and binding diagnostics. Capture mode uses an
offscreen target for deterministic GPU images, while interactive runs use the
normal window target.
Three additional `motion-*.png` captures and reports verify retained identity
and focus during reordering, insertion and removal.
