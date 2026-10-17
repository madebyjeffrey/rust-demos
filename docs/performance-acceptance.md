# Native performance and final acceptance — 2026-10-17

Ticket: [#13](https://github.com/madebyjeffrey/rust-demos/issues/13).

## Measurement protocol

All timing runs used development-signed Cargo release builds on the actual iPhone
17 Pro (iOS 27.0.1) and iPad mini (6th generation, iPadOS 27.0.1). Xcode 27.0
(27A266a), iPhoneOS 27.0 SDK, deployment target 27.0, Rust/Cargo 1.99.1,
Bevy 0.20.0, winit 0.30.13, and wgpu 30.0.1/Metal were used. The phone was wired;
the iPad used its paired local-network connection. Driver caches were retained.
Each launch measured startup separately, then 10 seconds of warmup and 60 seconds
of steady animation/interaction. The native XCTest driver ran for at least 75
seconds, took its screenshot after capture completed, and terminated the app.
No per-frame console transport or screenshots ran during measurement. Native
runner thermal state was nominal (0) at the beginning and end of accepted runs.

An opt-in app Documents capture avoided debugger/console attachment during the
measurement. Each file was retrieved after completion, checked for the expected
scenario, complete warmup/steady raw samples and final rendering dimensions.
Input counters establish that orbit and pinch actually reached the app; native
frame assertions and resize events establish real portrait/landscape changes.
The user-visible app was not driven by scripted camera transforms.

These are monotonic application-loop intervals, including pacing and rendering
backpressure. They are not direct GPU execution or displayed-frame timings.
`1000/mean_ms` is reported as cadence; every interval above the exact 1000/60 ms
budget is counted without rounding. Small overshoots at vsync do not by themselves
prove a visible dropped frame. Long intervals and cadence shortfalls are retained.
The interval crossing a phase boundary is assigned by its ending update, so the
sum of measured intervals differs slightly from the requested duration.

Scenarios: idle animation untouched; alternating slow right-side backdrop drags;
alternating 0.65×/1.54× pinches at ±0.5 velocity; repeated portrait and both
landscape orientations with a four-second dwell and native window-frame checks.

## Builds and rendering

The unchanged-resolution comparison is source `e3f5d14` (telemetry added to merged
main `c9bacb0`). Candidate A is source `50c6411`: scene rendering at 0.6 of native
width and height with a separate native-resolution UI composite. Both retain
64,000 main stars, 4,000 background stars, companion galaxies, 256×128×256 gas/dust
textures, 64 gas samples, 16 stellar-extinction samples, HDR, natural bloom,
4× scene MSAA and Fifo presentation. Candidate A uses 1× MSAA for the native UI
composite. No scene content was removed. Desktop stays at its original resolution.

Native portrait UI/window dimensions are 1206×2622 (402×874 logical, scale 3) on
iPhone and 1488×2266 (744×1133 logical, scale 2) on iPad. Candidate A scene targets
are 724×1573 and 893×1360 respectively; native labels and touch targets retain
their dimensions. Landscape swaps target dimensions. Visual captures retain the
galaxy backdrop's character with a softer scene and native-sharp labels.

## Results

The iPad's measured steady application cadence rose from 40–50 fps to
59.5–60 fps across the four scenarios. The iPhone rose from 44–46 fps to
51–56 fps. **The iPhone does not meet the 60 fps target.** Both devices retain
intervals above 16.667 ms, including paired short/long intervals on iPad. These
results do not certify a locked 60 fps presentation rate on either device.

The final source is `c0145e9`: reduced scene rendering plus an active-scene
`UIUpdateLink` requesting 60 Hz. The supported update action is registered before
continuous updates, and the link disables while inactive. This leaves Bevy/winit
in charge of rendering and input. No further effects were cut after the
resolution comparison: the independent Metal HUD spot check showed about
7.47 ms GPU work on iPhone, so further visual reductions were not justified by
that limited evidence. The HUD itself can influence cadence and was excluded
from every accepted timing run. A short windowed Instruments trace was only a
pacing investigation, not sustained-performance acceptance.

The complete startup, warmup/steady distributions, measured durations, rendering
configuration timelines, event counts and raw-file hashes are in
[the machine-readable results](performance-results-2026-10-17.json).
All following rows are 60-second requested steady phases, with warmup excluded.

| Device / scenario | Native fps | Final fps | Final frames / measured s | Mean ms | p50 ms | p95 ms | p99 ms | Max ms | >16.667 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |

| ipad / idle | 39.99 | 59.99 | 3600 / 60.007 | 16.669 | 16.641 | 28.981 | 29.561 | 30.903 | 1728 (48.00%) |
| ipad / orbit | 40.23 | 59.91 | 3596 / 60.019 | 16.691 | 16.654 | 27.714 | 29.641 | 34.708 | 1765 (49.08%) |
| ipad / pinch | 41.05 | 59.93 | 3597 / 60.023 | 16.687 | 16.662 | 18.997 | 29.577 | 36.323 | 1791 (49.79%) |
| ipad / layout | 50.45 | 59.51 | 3572 / 60.023 | 16.804 | 16.652 | 29.231 | 30.303 | 40.880 | 1742 (48.77%) |
| iphone / idle | 46.23 | 53.83 | 3232 / 60.042 | 18.577 | 16.697 | 33.138 | 52.382 | 254.319 | 1720 (53.22%) |
| iphone / orbit | 44.47 | 51.36 | 3082 / 60.013 | 19.472 | 16.754 | 34.163 | 42.467 | 64.382 | 1770 (57.43%) |
| iphone / pinch | 44.88 | 56.05 | 3364 / 60.013 | 17.840 | 16.710 | 32.818 | 35.036 | 47.140 | 1800 (53.51%) |
| iphone / layout | 45.63 | 51.77 | 3107 / 60.011 | 19.315 | 16.743 | 33.748 | 43.058 | 80.584 | 1804 (58.06%) |

Startup is plugin registration to first app update, rather than time to first
presented frame. Final startup ranged from 386.927–455.444 ms on iPad and
230.835–241.870 ms on iPhone (individual values below are authoritative).

| Device / scenario | Startup ms | Warmup frames / measured s | Warmup mean ms | Warmup p95 ms | Warmup max ms | Warmup >16.667 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ipad / idle | 390.105 | 584 / 9.996 | 17.117 | 29.093 | 134.943 | 289 (49.49%) |
| ipad / orbit | 386.927 | 587 / 9.992 | 17.023 | 26.326 | 87.591 | 280 (47.70%) |
| ipad / pinch | 393.190 | 581 / 9.989 | 17.192 | 22.651 | 116.457 | 295 (50.77%) |
| ipad / layout | 455.444 | 583 / 9.988 | 17.132 | 29.397 | 87.913 | 295 (50.60%) |
| iphone / idle | 241.870 | 587 / 9.987 | 17.013 | 18.078 | 63.069 | 301 (51.28%) |
| iphone / orbit | 234.402 | 574 / 9.998 | 17.419 | 27.562 | 98.652 | 303 (52.79%) |
| iphone / pinch | 230.835 | 592 / 9.995 | 16.883 | 24.459 | 42.726 | 304 (51.35%) |
| iphone / layout | 238.718 | 590 / 9.998 | 16.946 | 17.956 | 47.386 | 310 (52.54%) |

Resolution reduction alone (candidate A) produced 59.95 fps on iPad and
45.09 fps on iPhone during idle; those comparison distributions are retained in
the JSON. The final request improved the measured iPhone cadence but left its
shortfall and long intervals visible. An additional console-only iPhone
cross-check failed at the device transport boundary and produced no samples;
it is excluded. The accepted matrix used the native XCTest driver described
above. A single run per device/scenario is not a long-duration thermal study.

## Final hardware and regression acceptance

The final development-signed release installed and signature verification passed
on both baseline devices. Installed executable SHA-256 values:

- iPad: `a210690da1af8af9107dbfaf0128b5885863f7772e702ae0e2723877a7c40c27`.
- iPhone: `9b174312d146771877bdb9ae1bf4073b4f2123a5816cd828e233814e1a8687bc`.

The final native acceptance tests passed in 45.387 s on iPad and 41.561 s on
iPhone. All twenty retained captures were inspected: a backdrop drag changes
orbit, pinches change scale in both directions, a new single-finger drag works
after pinching, a drag starting inside New Game preserves camera framing, all
three placeholder controls keep the application foreground, and fresh input
works after background/reactivation. Portrait, both landscape directions and
portrait return retain readable controls inside the safe areas, a full-window
galaxy backdrop, no foreground planets or details UI, and no mobile Quit.
The native tests reject transient notification banners in acceptance captures.

The iPad multitasking test passed in 28.851 s. With the owner's permission,
Windowed Apps was enabled temporarily. The actual native window grew from
375×486 to 644×929 and shrank to 375×486 logical points; both captures show all
three controls and the backdrop covering the window. A native assertion verified
that the original Full Screen Apps setting was restored. Device orientation was
restored by each runner's cleanup block.

Desktop release smoke retained mouse activation, visible keyboard focus,
keyboard activation, wheel zoom, window zoom, and functional mouse/keyboard Quit.
A packaged GPU harness using the final shared plugin captured 1280×720,
320×200, 240×200, 600×240, 402×874 and the original 1280×720 window. All six
captures were inspected and retain all four readable controls and the backdrop.
The earlier unbundled harness's black captures were discarded before the
packaged repetition. App-level tests provide precise camera/orbit/relative-zoom,
input ownership, cancellation, scaling, safe-area and resizing assertions.

All 57 Rust workspace tests, both host capture-command tests, formatting, strict
all-target Clippy and native Objective-C syntax checks pass. The three new app
regressions cover native UI dimensions with reduced scene rendering, target
resizing/aspect and allocation stability, and native-coordinate menu/backdrop
input. No gameplay or working save/load/settings flows were added.

## Repeating the checks

Follow [ios.md](ios.md) for device selection, signing, installation and ordinary
launch. Build the release from the recorded source and run the [capture
workflow](performance-capture.md), preserving a separate output per device and
scenario. Drive repeated slow backdrop drags, alternating pinches and four-second
orientation dwells during the capture; validate `GALAXY_EVENTS` before accepting
any named scenario. Keep screenshots and profiler/HUD work outside timing runs.

A native test driver may launch with `GALAXY_FRAME_BASELINE=1`,
`GALAXY_BASELINE_FILE=1`, a unique `GALAXY_BASELINE_SCENARIO`, and 10/60-second
phase settings, then retrieve `Documents/galaxy-baseline.log` from the app's
`appDataContainer`. Verify a new complete report for that unique scenario;
a prior completed file remains until replaced. The temporary XCTest driver,
raw per-frame files, test results and screenshots are retained locally in
ignored `target/ios/performance-13/` in the main checkout. Raw files contain no
benchmark-time console transport; broader native-tool artifacts stay private
because they can include device/signing identifiers. Their hashes in the JSON
allow the retained frame files to be matched to these results.
