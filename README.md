# rust-demos

Bevy demos live under `apps/`; reusable Rust libraries live under `crates/`.

## Galaxy Menu

Launch the first demo with:

```sh
cargo run -p galaxy-menu
```

The menu shell presents placeholder actions for New Game, Load Game, Settings, and Quit.

## Composable UI prototype

```sh
cargo run -p composable-ui-prototype
```

A small native Bevy demo of composable Rust views, responsive display slots, and
thin or thick curved borders drawn by one shared frame owner. Resize the window
or use the on-screen controls to explore it. See
[the demo guide](apps/composable-ui-prototype/README.md) for controls and implementation details.

## Field Notes / garden UI

```sh
cargo run -p garden-ui
```

A light garden workspace with an asymmetric CSS Grid, sidebar, responsive
stacking, and bottom/right curved frame regions. It uses the same library as the
dark telemetry prototype. See [the garden guide](apps/garden-ui/README.md).

## Shared library

[bevy-composable-ui](crates/bevy-composable-ui) provides reusable Rust view
composition and anchored shared frame regions, including its embedded shader.
It also provides declarative resource bindings and keyed reconciliation that
preserve local state and focus across compatible updates and reordering.
Both UI demos depend on it through a Cargo path dependency. The
[library guide](crates/bevy-composable-ui/README.md) describes usage, geometry
contracts, limitations and validation commands.
