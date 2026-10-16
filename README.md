# rust-demos

Independent Bevy demos live in the workspace under `apps/`.

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

Build and directly install the same application on registered iPhone and iPad devices
with the [native iOS workflow](docs/ios.md).
