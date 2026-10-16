# Native launch baselines — 2026-10-15

Ticket: [Launch Galaxy Menu on iPhone and iPad (#8)](https://github.com/madebyjeffrey/rust-demos/issues/8).

## Physical iPad acceptance

The signed release bundle installed on the registered **iPad mini (6th generation),
iPadOS 27.0.1**. After fixing the iOS 27 scene-lifecycle startup assertion, a
physical-device screenshot showed the rendered galaxy, background stars, gas,
companions and existing menu together. The image filled the portrait display at
1488×2266 physical pixels (744×1133 logical, scale 2). This ticket preserves the
existing scene and placeholder UI; the other native-support tickets replace its
planet/card UI and implement touch/layout behavior.

The final baseline was a repeat launch after the first successful visual check.
Driver shader caches were not cleared. The app remained foreground, idle and
untouched throughout capture; no screenshot was taken during this final measured
run. The device was connected to the Mac; thermal state was not sampled.

Build: Cargo release, `aarch64-apple-ios`, deployment target 27.0, Xcode 27.0
(build 27A266a), iPhoneOS 27.0 SDK, Rust/Cargo 1.99.1, Bevy 0.20.0,
winit 0.30.13, wgpu 30.0.1/Metal. The implementation is based on repository commit
`72de5d5`, with the native launcher/composition/telemetry changes in this ticket.
The installed, development-signed executable's SHA-256 was:

```text
1f2dd17a4083895a4f405ba3430fe08a49b01b51e3c4cf633c91a70af86e3f9c
```

Rendering was unchanged: native 1488×2266 resolution, Fifo presentation,
4× MSAA, depth prepass, natural bloom, 64,000 main galaxy stars, 4,000 background
stars, companion galaxies, 256×128×256 gas and dust textures, and 64 gas ray-march
samples. No resolution or effects reduction was applied.

Startup from shared-plugin registration to the first app update: **397.314 ms**.
This excludes process launch and any work before plugin registration. The next
10 seconds were warm-up, followed by 60 seconds of steady idle animation.

| Phase | Frames | Mean ms | p50 ms | p95 ms | p99 ms | Maximum ms | Frames >16.667 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Warm-up | 389 | 25.618 | 30.047 | 37.974 | 50.404 | 155.573 | 202 (51.93%) |
| Steady idle | 2,402 | 24.996 | 23.190 | 36.942 | 44.609 | 49.341 | 1,220 (50.79%) |

The steady intervals totalled 60.040 seconds. This initial native-resolution result
**does not meet the 60 fps target**; its mean cadence is about 40 frames/second.
These are application frame-loop intervals, not direct GPU or presented-frame
timings. The baseline establishes an unchanged-quality starting point for later
profiling and tuning; it does not certify sustained interaction performance.

The repeatable capture procedure and metric definitions are in [ios.md](ios.md).
Raw per-frame samples were retained locally in ignored
`target/ios/ipad-baseline.log`; the verified physical screenshot was retained as
`target/ios/ipad-galaxy.png`. Those raw artifacts may be preserved separately from
worktree cleanup. Signing profiles, entitlements and device identifiers stay local.

## Physical iPhone acceptance

The development-signed release installed and launched on the registered
**iPhone 17 Pro, iOS 27.0.1**. A native screenshot verified the galaxy, gas,
background stars, companions and existing menu rendered together. The portrait
window settled at **1206×2622 physical pixels (402×874 logical, scale 3), Fifo**.
The existing menu overlap is outside this launch ticket; the sibling removal,
controls and responsive-layout tickets retain that work.

The build used the same Cargo release target, toolchain and unchanged rendering
settings listed above. Source commit `de25965` includes a telemetry correction:
window configuration is sampled at capture completion, after native resize/scale
events have settled, instead of reporting the default window before startup
synchronization. This changes metadata reporting only; the scene, effects and
rendering resolution policy are unchanged from the iPad build. The installed,
development-signed executable's SHA-256 was:

```text
b948539d41334cb00872a2bec8090551a9d8275e5f9dcc2d1e6aec484ad5bcb6
```

This was a repeat launch after verifying visible rendering. Driver shader caches
were not cleared. Capture used idle foreground animation with no injected input or
screenshots during measurement. The phone was connected to the Mac; thermal state
was not sampled. Startup from shared-plugin registration to the first app update
was **145.388 ms**, followed by 10 seconds of warm-up and 60 seconds of steady
capture. The raw steady intervals totalled **60.026 seconds**.

| Phase | Frames | Mean ms | p50 ms | p95 ms | p99 ms | Maximum ms | Frames >16.667 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Warm-up | 581 | 17.198 | 16.714 | 21.193 | 36.879 | 53.268 | 301 (51.81%) |
| Steady idle | 2,611 | 22.990 | 17.194 | 36.082 | 39.388 | 51.728 | 1,559 (59.71%) |

This initial native-resolution baseline **does not meet the 60 fps target**; its
mean application-loop cadence is about 43.5 frames/second. As with the iPad,
these are frame-loop intervals, not direct GPU execution or presented-frame
timings. No resolution or effects reduction was applied. Final interaction
profiling and tuning remain in ticket #13.

The final raw capture and verified screenshot were retained locally in ignored
`target/ios/iphone-baseline.log` and `target/ios/iphone-galaxy.png`, with a private
copy preserved separately from worktree cleanup. Ordinary launch was checked again
after measurement and the application was left open on the phone.

## Launch acceptance complete

Both registered physical devices now have signed release installation, visible
galaxy rendering and separate warm-up/steady baselines recorded. Together with the
shared-application regression and desktop smoke below, this completes ticket #8's
launch acceptance. Touch gestures, responsive layout, menu controls and final
60 fps tuning remain in the sibling native-support tickets.

## Desktop release smoke

The local macOS host built the same application successfully with
`cargo build --release --locked --offline -p galaxy-menu`. A live release-app
screenshot showed the galaxy, gas, companions and menu, and mouse drag orbit and
wheel zoom remained responsive. The opt-in 70-second desktop cadence capture also
completed at 1280×720 physical pixels, scale 1 and Fifo presentation. This is
desktop smoke evidence; it is separate from the physical-device measurements above.
