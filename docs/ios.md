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

Physical results and remaining device acceptance are recorded in
[native launch baseline](ios-baseline.md). Touch input, responsive layout, replacement
menu controls and the final 60 fps target belong to the remaining native-support
tickets; the initial launch intentionally retains the current desktop scene/menu.

## Desktop smoke

```sh
cargo test --workspace
cargo run -p galaxy-menu
# Optional bounded desktop frame-cadence smoke (same capture protocol):
GALAXY_FRAME_BASELINE=1 cargo run -p galaxy-menu --release
```

The shared-application integration test assembles the real scene and menu, checks
that they are present, sends mouse drag input and observes camera movement.
Existing scene, menu, camera and display-scale regressions continue to run.
