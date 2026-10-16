# Native launch baseline — 2026-10-15

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

## Remaining physical acceptance

**iPhone 17 Pro / iOS 27.0.1: pending.** The local development profile includes the
registered phone, but no phone installation, visible-rendering check or frame-time
capture is claimed. The owner directed this run to use the available iPad for now.
Repeat the documented build/install/launch/visual/baseline workflow when the phone
is available. Ticket #8 remains open until that acceptance is complete.

Touch gestures, responsive layout, menu controls and final 60 fps tuning remain
in the sibling native-support tickets.
