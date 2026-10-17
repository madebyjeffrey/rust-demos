# Repeatable release frame captures

Performance telemetry is opt-in and has no product controls. The existing
`GALAXY_FRAME_BASELINE=1` launch still captures 10 seconds of warmup followed by
60 seconds of steady frames. Frame intervals use a monotonic wall clock between
application updates; they include scheduling and presentation pacing and are
not isolated GPU timings.

Name a scenario and keep its raw capture separate:

```sh
python3 scripts/ios.py baseline --scenario orbit \
  --warmup-seconds 10 --steady-seconds 60 \
  --output target/ios/orbit.log
```

The command launches the already installed signed executable. Drive the named
scenario during its steady phase. A name describes intent; inspect `GALAXY_EVENTS`
to establish that touches or window changes actually reached the application.
Do not treat an idle capture labeled `orbit` as interaction evidence.

The equivalent environment variables work for desktop launches and native test
harnesses:

| Variable | Default | Accepted values |
| --- | --- | --- |
| `GALAXY_FRAME_BASELINE` | disabled | `1` enables capture |
| `GALAXY_BASELINE_SCENARIO` | `baseline` | 1–64 letters, digits, underscores or hyphens, beginning with a letter or digit |
| `GALAXY_BASELINE_WARMUP_SECONDS` | `10` | finite seconds from `0.1` through `600` |
| `GALAXY_BASELINE_STEADY_SECONDS` | `60` | finite seconds from `0.1` through `600` |
| `GALAXY_BASELINE_FILE` | disabled | `1` also saves `$HOME/Documents/galaxy-baseline.log` |

Choose phase lengths that contain enough frames on the measured hardware. An
extremely short phase can contain no samples during startup stalls; the host
command rejects captures without warmup and steady raw frames. The host timeout
allows the requested capture duration plus 50 seconds for launch and transport.

Each completed report includes:

- `GALAXY_CAPTURE`: scenario and requested phase durations.
- `GALAXY_STARTUP`: time from telemetry registration to the first application
  update. This excludes process initialization before registration and does not
  claim to be the time to first presented frame.
- `GALAXY_CONFIG`: elapsed time for each distinct rendering configuration. Native
  window pixels, logical size, and display scale are recorded separately from
  the scene camera's physical viewport and render target dimensions. Bloom,
  MSAA samples, HDR, tonemapping, and presentation mode describe that camera.
- `GALAXY_SUMMARY`: warmup and steady sample counts, measured interval durations,
  mean, p50/p95/p99/max, and count/percentage above the 60 Hz frame budget.
- `GALAXY_FRAME`: every buffered interval, its phase, elapsed capture seconds,
  and frame milliseconds. The interval crossing a phase boundary belongs to the
  phase in which its ending update occurs.
- `GALAXY_EVENTS`: counts by phase for touch start/move/end/cancel, mouse, wheel,
  keyboard, resize, scale, and focus events received through the window boundary.
- `GALAXY_COMPLETE`: the report finished.

Formatting and file/console I/O happen after measurement completes. Native
harnesses can enable the file to drive gestures without an attached console,
then retrieve the application's Documents file. A file-write failure emits
`GALAXY_ERROR`. Ordinary launches do not create a capture file. The iOS process
stays active after capture; the console command stops it after completion.
Desktop captures request application exit after reporting.

The shared `GalaxyRenderSettings` resource controls scene resolution independently
of the native window. Its platform default is `scene_scale: 0.6` on iOS and
`1.0` on desktop. At reduced scale, the galaxy camera renders to an image and a
native camera composites that image with native UI. Scene settings such as bloom,
MSAA, stars, gas, dust, and companions are retained. Resizing updates the image
only when its pixel dimensions change. Insert the resource before the shared app
plugin, or update it when comparing rendering policies in an acceptance harness.

Native scene pacing uses a scene-owned `UIUpdateLink` with continuous updates
and a preferred 60 Hz range while the scene is active. It is disabled when the
scene resigns active and released on disconnect. The link supplies timing
preferences; winit and Bevy keep render and input ownership. This is a measured
candidate, not a guarantee that iOS will deliver the requested rate. Apple
[documents UIUpdateLink preferences separately from optional callbacks](https://developer.apple.com/documentation/uikit/uiupdatelink),
and [requiresContinuousUpdates](https://developer.apple.com/documentation/uikit/uiupdatelink/requirescontinuousupdates)
requests ongoing UIKit updates. Physical captures determine whether the request
improves sustained delivery. The
[phone high-refresh plist key](https://developer.apple.com/documentation/bundleresources/information-property-list/cadisableminimumframedurationonphone)
unlocks rates above the system default; a 60 Hz request does not need that opt-in.

Run the host capture command regressions without a device:

```sh
python3 -m unittest discover -s scripts/tests -v
```
