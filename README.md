# rust-demos

Independent Bevy demos live in the workspace under `apps/`.

## Galaxy Menu

Launch the first demo with:

```sh
cargo run -p galaxy-menu
```

The menu shell presents placeholder actions for New Game, Load Game, Settings, and Quit.
The galaxy backdrop fills the window behind the menu, with stars, gas, dust and
orbiting companions. Drag with the left mouse button to orbit continuously through
the poles and use the mouse wheel to zoom within the scene's framing bounds.
Menu overlays share the full window without reserving a separate scene viewport.

Build and directly install the same application on registered iPhone and iPad devices
with the [native iOS workflow](docs/ios.md).
