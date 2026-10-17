# Native Galaxy Menu

Desktop and iOS run the same `main.rs`, `GalaxyMenuPlugin`, scene, shaders and menu.
The native package contains Cargo's ARM64 executable; it does not require a separate
Swift scene implementation or assets downloaded at runtime. WESL shaders and the
font are embedded, and galaxy textures and geometry are generated locally.

## Prerequisites

Use macOS with Xcode 27 selected, its iOS 27 SDK, Python 3, and Rust's
`aarch64-apple-ios` target:

```sh
rustup target add aarch64-apple-ios
xcodebuild -version
```

Register the device with the local Apple development team, pair it in Xcode,
enable Developer Mode, and unlock it. The local login keychain must contain the
Apple Development identity's private key. Xcode must have downloaded a valid
**development** provisioning profile for `dev.rustdemos.galaxy-menu` (or a matching
wildcard), including the registered device. This workflow reuses local provisioning;
it does not create an Apple account, register devices, or distribute through the
App Store. When provisioning changes, refresh the profile through Xcode.

## Build, install and launch

Run these from the repository root. Obtain the desired device's identifier or name
from Xcode's Devices and Simulators, or `xcrun devicectl list devices`. Keep that
personal value in the shell environment; do not put it in source or documentation.

```sh
# Paste the selected device identifier/name into the prompt, then press Return.
read -r IOS_DEVICE
export IOS_DEVICE
python3 scripts/ios.py build
python3 scripts/ios.py install
python3 scripts/ios.py launch
python3 scripts/ios.py screenshot
```

The build uses Cargo's **release** configuration, deployment target iOS 27.0, and
the current iPhoneOS SDK. `CARGO_TARGET_DIR` may point to an existing build cache.
The signed app, entitlements, embedded profile, logs and screenshot are written to
ignored `target/ios/`. The script selects an unexpired matching development profile
that includes the selected device and whose certificate has a valid local signing
identity/private key. To select a profile explicitly, set
`IOS_PROVISIONING_PROFILE` to its local `.mobileprovision` path before building.
`codesign --verify --strict` must succeed before the bundle is reported ready.

Build once and install on each registered device whose UDID is included in the
embedded profile. Build again with the other `IOS_DEVICE` when its provisioning
profile differs. Raw Xcode/device errors stay in `target/ios/last-error.log` because
they may contain personal identifiers. The screenshot is
`target/ios/screenshot.png`; inspect it to verify the running app rather than relying
only on a successful launch request.

### iOS 27 startup bridge

UIKit requires the scene lifecycle for applications linked with the iOS 27 SDK.
The current Bevy dependency uses winit 0.30, which creates its own legacy UIWindow.
`ios/scene.m` supplies a single `UIWindowSceneDelegate` and attaches **that existing
winit window** to the connected scene. winit continues to own its view, rendering,
input and event loop. The scene manifest supports iPhone and iPad, rotation and
window resizing, with multiple independent app scenes disabled.

Without this bridge the physical iPad terminated before Bevy's first frame with
`___UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption_block_invoke`.
Remove the bridge once the windowing dependency provides native scene support.
Apple describes the requirement and lifecycle migration in
[Transitioning to the UIKit scene-based life cycle](https://developer.apple.com/documentation/uikit/transitioning-to-the-uikit-scene-based-life-cycle).

## Initial frame-time baseline

```sh
python3 scripts/ios.py baseline
# Preserve each device's capture locally before switching devices or repeating.
cp target/ios/baseline.log target/ios/ipad-baseline.log
```

This launches the installed signed release app with `GALAXY_FRAME_BASELINE=1`.
Keep it foreground and untouched for the initial idle baseline. Startup is reported
separately as time from application plugin assembly to the first app update. The
next 10 seconds cover renderer/shader warm-up; the following 60 seconds are steady
idle animation. The app then prints summaries and raw samples; the host command stops the
console-attached app after its completion marker. Missing
either phase makes the command fail. Ordinary launches do not enable telemetry.

Measurements use monotonic wall-clock intervals between app `Last` schedules.
They describe **application frame-loop cadence**, including pacing and CPU/GPU
backpressure; they are not direct GPU execution or display presentation timings.
Samples are buffered until capture finishes to keep per-frame console transport
out of the measurement. The capture reports sample count, mean, p50/p95/p99,
maximum and the number/percentage over the exact `1000/60` ms budget. Tiny timing
variations near that threshold do not independently prove a visible dropped frame.
Startup and warm-up are excluded from the steady distribution. Window configuration
is logged at capture completion, after native window sizing has replaced the startup
defaults.

The initial scene is unchanged: native display resolution, Fifo presentation,
4× MSAA, depth prepass, natural bloom, 64,000 main galaxy stars, 4,000 background
stars and companion galaxies; gas and dust use 256×128×256 textures and a 64-step
gas ray march. There is no mobile quality reduction in this ticket. Later device
profiling and tuning can compare against this baseline. Record device/OS, rendering
settings, startup, both measurement phases, source/build identity and interaction
scenario when repeating it. Instruments' Game Performance or Metal System Trace
can corroborate actual presentation and GPU behavior.

Physical results and device acceptance are recorded in
[native launch baseline](ios-baseline.md), which describes the initial launch
milestone. The shared startup menu now provides tap controls for New Game, Load
Game, and Settings, with Quit omitted on iPhone and iPad. Touch camera gestures now share the desktop camera behavior; their verification
status is recorded below. The full responsive layout and final 60 fps target
remain in the later native-support tickets.

## Desktop smoke

```sh
cargo test --workspace
cargo run -p galaxy-menu
# Optional bounded desktop frame-cadence smoke (same capture protocol):
GALAXY_FRAME_BASELINE=1 cargo run -p galaxy-menu --release
```

The shared-application integration test assembles the real scene and menu, checks
that they are present, sends mouse drag input and observes camera movement.
Menu integration tests send real pointer, touch, keyboard, and focus events and
check feedback, activation, desktop exit, and camera ownership. Existing scene,
menu, camera and display-scale regressions continue to run.


## Menu-controls acceptance — 2026-10-16

[Ticket #10](https://github.com/madebyjeffrey/rust-demos/issues/10) was checked
with the shared application at source commit `3b289a2` (integrated in `89657f9`).
Desktop and `aarch64-apple-ios` release builds succeeded. The development-signed
native executable installed on both baseline devices had SHA-256:

```text
022ce0040a3b7f4169848054e08b8e35dad6fa02fa81e3c5a35d3e3cf2fe65e9
```

| Physical device | OS | Rendered window | Controls smoke |
| --- | --- | --- | --- |
| iPhone 17 Pro | iOS 27.0.1 | 1206×2622, scale 3 | Pass |
| iPad mini (6th generation) | iPadOS 27.0.1 | 1488×2266, scale 2 | Pass |

A temporary, locally signed XCTest UI runner launched the installed app by bundle
identifier. On each device it held New Game, Load Game and Settings for four
seconds each, tapped each control, dragged from New Game into surrounding space,
and tapped again afterward. Both native test runs passed and kept the placeholder
menu foreground. Settled screenshots showed the galaxy backdrop, all three
readable controls and no Quit. Separate screenshots taken during each hold
confirmed pressed feedback on every control on both devices. Test bundles, raw
results, screenshots and the signed app remain local; personal device identifiers
and signing material are excluded from the repository.

To repeat the smoke check, build, install and launch using the commands above;
tap and hold each control, drag a control press into the surrounding space, and
check that the next tap works. New Game, Load Game and Settings keep their
placeholder behavior. Inspect the settled display after renderer startup and
confirm Quit is absent.

The macOS release smoke verified all four controls, pointer hover and activation,
visible keyboard focus, Tab navigation with Enter/Space activation, control drag
capture, and backdrop drag/wheel navigation. Both mouse and keyboard activation
of Quit terminated the process. App-level regressions separately cover
cancellation, focus loss, display scaling, batched input, complete taps received
between frames, and independent touches on the same control. All 31 repository
tests, formatting, and strict Clippy passed; the original four independent review
reproductions also passed after the fixes. Standards and spec review have no
remaining actionable findings.

This acceptance covers the provisional controls slice. Touch camera gestures,
full responsive layout and physical-device performance tuning continue in
[#11](https://github.com/madebyjeffrey/rust-demos/issues/11),
[#12](https://github.com/madebyjeffrey/rust-demos/issues/12) and
[#13](https://github.com/madebyjeffrey/rust-demos/issues/13).


## Touch-navigation acceptance — 2026-10-17

[Ticket #11](https://github.com/madebyjeffrey/rust-demos/issues/11) is implemented
in [PR #18](https://github.com/madebyjeffrey/rust-demos/pull/18). One backdrop
finger orbits continuously through the poles; two backdrop fingers change bounded
zoom. Adding, removing or cancelling fingers rebases the remaining gesture.
Focus loss and application suspension discard stale tracking. A touch beginning
on a control retains UI ownership even outside its bounds; independent backdrop
touches can still orbit or pinch around it. Galaxy and companion idle motion
pauses while touches are held, then resumes after release. Mouse drag and wheel
zoom continue to use the existing camera behavior.

All 50 workspace tests pass, including 11 new assembled-app regressions supplying
real touch, window-focus and lifecycle messages. They cover movement without
replaying held deltas, zoom bounds, pole crossings, finger transitions,
cancellation, reused touch IDs and ordered event batches, UI/backdrop ownership,
and galaxy/companion idle coordination. Formatting, strict all-target Clippy and
diff checks pass. The independent standards and spec reviews have no remaining
actionable findings after sharing the orbit and interruption-reset logic.

The reviewed source at `00449ba` builds a development-signed ARM64 iOS release.
Signature verification and installation succeeded on both registered baseline
devices. The installed executable has SHA-256:

```text
048aaf2a882bbc42fa1b01f27345734016ec94b7f6969666649af854dc232a80
```

| Physical device | OS | Signed release | Gesture acceptance |
| --- | --- | --- | --- |
| iPhone 17 Pro | iOS 27.0.1 | Installed and verified | Pass (2026-10-17) |
| iPad mini (6th generation) | iPadOS 27.0.1 | Installed and verified | Pass (2026-10-16) |

The iPad was released after the independent #12 layout checks. Before installing
#11 again, the signed bundle's executable hash was verified against the value
above. A temporary signed XCTest runner then launched that #11 build in portrait
and completed the native gesture sequence in 40.410 seconds with zero failures:
a backdrop drag, three further vertical drags through the poles, pinch in both
directions, a fresh single-finger orbit after pinching, a drag beginning on New
Game followed by a fresh tap, and background/reactivation followed by another
orbit. Eight retained screenshots were inspected: camera orbit changes the
backdrop, pinch changes its scale, the UI-owned drag preserves camera framing,
and the final fresh drag responds after interruption. The temporary iPad runner
was removed and the reviewed #11 app was left foreground. The #12 binary and
results were not used as #11 gesture evidence.

The iPhone became available on 2026-10-17. Device access was coordinated with
#12: #11 used exclusive slots, then explicitly handed the phone back for #12's
independent layout checks and final installation. The same verified #11 signed
binary completed the final native gesture sequence in 44.887 seconds with zero
failures. All eight whole-screen captures were inspected without notification
overlays. They show orbit and pole-crossing changes, pinch scaling in both
directions, a fresh orbit after pinching, unchanged camera framing during the
UI-owned drag, and a working new orbit after background/reactivation. The runner
restored the original device orientation and was removed before handback.

An earlier iPhone run had a transient system notification intercept several
gestures despite the runner reporting success; that run was excluded from
acceptance. The clean runner dismissed notification banners before gestures and
rejected captures when a banner interrupted a gesture. No notification contents
or personal device/signing identifiers are part of the acceptance record.

Both baseline devices now pass physical gesture acceptance for #11. PR #18 is
ready for review and closes the issue when merged. The #12 layout build and its
results remain separate from this touch-navigation evidence.

To repeat acceptance after unlocking the devices and authorizing native UI
automation, launch the installed release in portrait and inspect the rendered
backdrop. Drag in unused space, continue vertical drags through the poles, pinch
in both directions, then orbit again. Add and remove a pinch finger and cancel a
gesture using a system interruption; the next gesture must resume without a
jump. Drag from a menu control into the backdrop and confirm that the camera
stays still while the UI owns the gesture, then verify a fresh backdrop gesture.
Background and reactivate the app, and verify fresh orbit and menu taps. Repeat on both devices
and record settled before/after screenshots and the source/binary identity.
Temporary native test results and signing material stay local under ignored
`target/ios/` or the temporary acceptance directory.
## Responsive-layout acceptance — 2026-10-16

[Ticket #12](https://github.com/madebyjeffrey/rust-demos/issues/12) is implemented
at source commit `2d1c76c`, integrated in `f45a831`. The menu responds to logical
window size and UIKit safe-area insets; the camera preserves orbit and bounded
zoom relative to the full-window galaxy fit. Controls overlay the backdrop.
Desktop windows enforce a 240×200 logical-point minimum. Compact presentation
retains readable labels and at least 44-point control targets.

All 42 workspace tests, formatting, strict all-target Clippy, Objective-C syntax,
desktop release and signed `aarch64-apple-ios` release builds passed. Independent
standards and spec reviews have no remaining code findings. The review reproduced
an off-screen Quit at 320×200 before the final fix; that case and adjacent short,
narrow and safe-area configurations now pass at 1×/2×/3× display scales.
App-level input regressions retain desktop controls, ownership and full-window
framing. Repeated resize sequences preserve orbit, relative zoom and the original
camera position on returning to the initial dimensions.

An out-of-repository rendering harness assembled `GalaxyMenuPlugin` with the
production plugin configuration and drove actual desktop window resizes through
1280×720, 320×200, 240×200, 600×240, 402×874 and back to 1280×720. GPU screenshots
at each settled size show all four readable controls and the full-window galaxy
backdrop. The desktop release also rendered during native window zoom.

The development-signed executable used on both baseline devices has SHA-256:

```text
1326d067e10d71062c8bb4ac7e851c5c57f8bb471eab72976a0fc8c191b6203a
```

On the physical iPad mini (6th generation), iPadOS 27.0.1, sequential XCTest
checks verified portrait 744×1133, both landscape directions 1133×744 and a return
to portrait, with actual native-window frame assertions and whole-screen captures.
With the owner's approval, Windowed Apps was enabled temporarily. The same app
grew from 375×486 to 644×929 and shrank back to 375×486 logical points over Settings.
Screenshots confirm all three readable controls, no Quit, no clipping and a galaxy
backdrop covering each native window. The test explicitly verified restoration of
the original Full Screen Apps setting. Initial investigation also observed the
full-screen 744×1133 to 375×551 window transition. These are layout checks, not
touch-navigation or performance acceptance.

To repeat native acceptance, build/install with the workflow above; rotate through
portrait and both landscape directions without restarting. In Windowed Apps,
resize with the bottom-right handle, checking both larger and smaller windows,
then restore the prior multitasking setting. Check control bounds, readable labels,
system-safe placement and the full-window backdrop at every transition. The
temporary XCTest runner, screenshots, raw results and signing material remain
local and are excluded from source control.

The iPhone was unavailable on 2026-10-16. Its remaining physical acceptance
was completed on 2026-10-17, as recorded below.


## iPhone responsive-layout acceptance — 2026-10-17

The remaining #12 hardware checks passed on the physical iPhone 17 Pro,
iOS 27.0.1, using the same verified development-signed executable and SHA-256
recorded above. Phone access was coordinated exclusively with the independent
#11 gesture-validation chat to avoid overwriting either build during its run.

The native orientation XCTest passed in 37.683 seconds with zero failures. Actual
window-frame assertions verified 402×874 logical points in portrait, 874×402 in
both landscape directions, and a return to 402×874 portrait, at display scale 3.
All four whole-screen screenshots were inspected: New Game, Load Game and
Settings remain readable and reachable inside the current safe areas, Quit is
absent, and the galaxy backdrop fills the window. The captures have no system
notification overlays. App-level regressions provide the precise control-bound,
display-scaling and repeated camera-continuity checks described above.

A separate cleanup XCTest verified restoration of the phone's original face-up
orientation (raw value 5), and the temporary layout test runner was removed.
Raw results and screenshots remain local, with device identifiers and signing
material excluded from source control. Both baseline devices now have completed
physical layout acceptance for #12; PR #19 can proceed to review. This record
does not claim #11 gesture acceptance or #13 performance acceptance.

## Touch and responsive-layout integration — 2026-10-17

The combined #18/#19 code uses relative zoom for both pinch and mouse-wheel
input, applying their shared bounds after all input for the frame. This resolves
the removed `GalaxyCamera.distance` reference and keeps pinch zoom bounded even
when no wheel event arrives. Both independent physical acceptance records above
are retained.

All 54 combined workspace tests pass, including a new input-driven regression
that compares pinch and wheel zoom bounds through repeated portrait, landscape,
multitasking and original-window transitions at 1×/2×/3× display scales. That
regression failed with a field rename alone and passed after sharing the clamp.
Formatting, strict all-target Clippy and the `aarch64-apple-ios` release build
also pass. Physical-device acceptance was not rerun for this integration change;
the hardware evidence above applies to the recorded independent builds.
