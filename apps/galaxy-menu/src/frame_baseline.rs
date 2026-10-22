//! Opt-in release acceptance telemetry. It has no product UI and is inactive by default.
use std::{fmt::Write as _, path::PathBuf, time::Instant};

use bevy::{
    app::AppExit,
    camera::Hdr,
    input::touch::TouchPhase,
    post_process::bloom::Bloom,
    prelude::*,
    render::view::Tonemapping,
    window::{PresentMode, PrimaryWindow, WindowEvent},
};

struct CaptureSettings {
    scenario: String,
    warmup_seconds: f64,
    steady_seconds: f64,
    file: Option<PathBuf>,
}

impl CaptureSettings {
    fn from_environment() -> Self {
        let seconds = |name, default| match std::env::var(name) {
            Ok(value) => value
                .parse::<f64>()
                .ok()
                .filter(|seconds| seconds.is_finite() && (0.1..=600.0).contains(seconds))
                .unwrap_or_else(|| panic!("{name} must be between 0.1 and 600 seconds")),
            Err(_) => default,
        };
        let scenario = std::env::var("GALAXY_BASELINE_SCENARIO").unwrap_or("baseline".into());
        assert!(
            !scenario.is_empty()
                && scenario.len() <= 64
                && scenario.as_bytes()[0].is_ascii_alphanumeric()
                && scenario
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'),
            "GALAXY_BASELINE_SCENARIO must be a 1–64 character alphanumeric, underscore or hyphen name"
        );
        Self {
            scenario,
            warmup_seconds: seconds("GALAXY_BASELINE_WARMUP_SECONDS", 10.0),
            steady_seconds: seconds("GALAXY_BASELINE_STEADY_SECONDS", 60.0),
            file: (std::env::var("GALAXY_BASELINE_FILE").as_deref() == Ok("1")).then(|| {
                PathBuf::from(std::env::var_os("HOME").expect("baseline file capture needs HOME"))
                    .join("Documents/galaxy-baseline.log")
            }),
        }
    }
}

#[derive(Default)]
struct EventCounts {
    touch_started: usize,
    touch_moved: usize,
    touch_ended: usize,
    touch_cancelled: usize,
    mouse_button: usize,
    mouse_motion: usize,
    mouse_wheel: usize,
    keyboard: usize,
    window_resized: usize,
    scale_changed: usize,
    focus_changed: usize,
}

impl EventCounts {
    fn record(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::TouchInput(input) => match input.phase {
                TouchPhase::Started => self.touch_started += 1,
                TouchPhase::Moved => self.touch_moved += 1,
                TouchPhase::Ended => self.touch_ended += 1,
                TouchPhase::Canceled => self.touch_cancelled += 1,
            },
            WindowEvent::MouseButtonInput(_) => self.mouse_button += 1,
            WindowEvent::MouseMotion(_) => self.mouse_motion += 1,
            WindowEvent::MouseWheel(_) => self.mouse_wheel += 1,
            WindowEvent::KeyboardInput(_) => self.keyboard += 1,
            WindowEvent::WindowResized(_) => self.window_resized += 1,
            WindowEvent::WindowScaleFactorChanged(_)
            | WindowEvent::WindowBackendScaleFactorChanged(_) => self.scale_changed += 1,
            WindowEvent::WindowFocused(_) => self.focus_changed += 1,
            _ => {}
        }
    }

    fn report(&self, phase: &str, output: &mut String) {
        writeln!(output,
            "GALAXY_EVENTS {phase} touch_started={} touch_moved={} touch_ended={} touch_cancelled={} mouse_button={} mouse_motion={} mouse_wheel={} keyboard={} window_resized={} scale_changed={} focus_changed={}",
            self.touch_started, self.touch_moved, self.touch_ended, self.touch_cancelled,
            self.mouse_button, self.mouse_motion, self.mouse_wheel, self.keyboard,
            self.window_resized, self.scale_changed, self.focus_changed,
        ).unwrap();
    }
}

#[derive(PartialEq)]
struct RenderSettings {
    native: UVec2,
    logical: Vec2,
    scale: f32,
    present_mode: PresentMode,
    scene: UVec2,
    target: UVec2,
    bloom: bool,
    msaa_samples: u32,
    hdr: bool,
    tonemapping: Option<Tonemapping>,
}

#[derive(Resource)]
struct FrameBaseline {
    settings: CaptureSettings,
    started: Instant,
    first_frame: Option<Instant>,
    first_update_ms: f64,
    previous: Instant,
    warmup: Vec<(f64, f64)>,
    steady: Vec<(f64, f64)>,
    warmup_events: EventCounts,
    steady_events: EventCounts,
    configurations: Vec<(f64, RenderSettings)>,
    complete: bool,
}

pub(super) fn register(app: &mut App) {
    if std::env::var("GALAXY_FRAME_BASELINE").as_deref() != Ok("1") {
        return;
    }
    let now = Instant::now();
    app.insert_resource(FrameBaseline {
        settings: CaptureSettings::from_environment(),
        started: now,
        first_frame: None,
        first_update_ms: 0.0,
        previous: now,
        warmup: Vec::new(),
        steady: Vec::new(),
        warmup_events: EventCounts::default(),
        steady_events: EventCounts::default(),
        configurations: Vec::new(),
        complete: false,
    })
    .add_systems(Last, capture_frame);
}

#[expect(
    clippy::type_complexity,
    reason = "Capture the scene camera and optional render settings together"
)]
fn capture_frame(
    mut baseline: ResMut<FrameBaseline>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<
        (
            &Camera,
            Option<&Bloom>,
            Option<&Msaa>,
            Has<Hdr>,
            Option<&Tonemapping>,
        ),
        With<Camera3d>,
    >,
    mut events: MessageReader<WindowEvent>,
    mut exit: MessageWriter<AppExit>,
) {
    if baseline.complete {
        return;
    }
    let now = Instant::now();
    let elapsed = baseline
        .first_frame
        .map_or(0.0, |first| now.duration_since(first).as_secs_f64());
    let counts = if elapsed < baseline.settings.warmup_seconds {
        &mut baseline.warmup_events
    } else {
        &mut baseline.steady_events
    };
    for event in events.read() {
        counts.record(event);
    }
    if let (Ok(window), Ok((camera, bloom, msaa, hdr, tonemapping))) =
        (window.single(), camera.single())
        && let (Some(scene), Some(target)) = (
            camera.physical_viewport_size(),
            camera.physical_target_size(),
        )
    {
        let settings = RenderSettings {
            native: window.physical_size(),
            logical: Vec2::new(window.width(), window.height()),
            scale: window.scale_factor(),
            present_mode: window.present_mode,
            scene,
            target,
            bloom: bloom.is_some(),
            msaa_samples: msaa.map_or(1, Msaa::samples),
            hdr,
            tonemapping: tonemapping.copied(),
        };
        if baseline
            .configurations
            .last()
            .is_none_or(|(_, previous)| *previous != settings)
        {
            baseline.configurations.push((elapsed, settings));
        }
    }
    if baseline.first_frame.is_none() {
        baseline.first_update_ms = now.duration_since(baseline.started).as_secs_f64() * 1000.0;
        baseline.first_frame = Some(now);
        baseline.previous = now;
        return;
    }
    let ms = now.duration_since(baseline.previous).as_secs_f64() * 1000.0;
    baseline.previous = now;
    if elapsed < baseline.settings.warmup_seconds {
        baseline.warmup.push((elapsed, ms));
    } else {
        baseline.steady.push((elapsed, ms));
    }
    if elapsed >= baseline.settings.warmup_seconds + baseline.settings.steady_seconds {
        // All formatting and I/O happen after measurement to avoid transport stalls.
        let output = baseline.report();
        if let Some(path) = &baseline.settings.file {
            let result = std::fs::create_dir_all(path.parent().unwrap())
                .and_then(|()| std::fs::write(path, &output));
            if let Err(error) = result {
                eprintln!("GALAXY_ERROR baseline_file_write={error}");
            }
        }
        print!("{output}");
        baseline.complete = true;
        // winit intentionally ignores application exit on iOS. The host baseline
        // command stops its console-attached process after receiving this marker.
        if !cfg!(target_os = "ios") {
            exit.write(AppExit::Success);
        }
    }
}

impl FrameBaseline {
    fn report(&self) -> String {
        let mut output = String::new();
        writeln!(
            output,
            "GALAXY_CAPTURE scenario={} warmup_seconds={} steady_seconds={}",
            self.settings.scenario, self.settings.warmup_seconds, self.settings.steady_seconds
        )
        .unwrap();
        writeln!(
            output,
            "GALAXY_STARTUP first_update_ms={:.3}",
            self.first_update_ms
        )
        .unwrap();
        for (elapsed, settings) in &self.configurations {
            writeln!(output,
                "GALAXY_CONFIG elapsed_seconds={elapsed:.6} physical_width={} physical_height={} logical_width={} logical_height={} scale={} present_mode={:?} scene_width={} scene_height={} target_width={} target_height={} bloom={} msaa_samples={} hdr={} tonemapping={:?}",
                settings.native.x, settings.native.y, settings.logical.x, settings.logical.y,
                settings.scale, settings.present_mode, settings.scene.x, settings.scene.y,
                settings.target.x, settings.target.y, settings.bloom, settings.msaa_samples,
                settings.hdr, settings.tonemapping,
            ).unwrap();
        }
        report("warmup", &self.warmup, &mut output);
        report("steady", &self.steady, &mut output);
        self.warmup_events.report("warmup", &mut output);
        self.steady_events.report("steady", &mut output);
        writeln!(output, "GALAXY_COMPLETE").unwrap();
        output
    }
}

fn report(phase: &str, samples: &[(f64, f64)], output: &mut String) {
    let mut sorted = samples.iter().map(|(_, ms)| *ms).collect::<Vec<_>>();
    sorted.sort_by(f64::total_cmp);
    if sorted.is_empty() {
        return;
    }
    let percentile = |p: f64| sorted[((sorted.len() - 1) as f64 * p).ceil() as usize];
    let missed = sorted.iter().filter(|ms| **ms > 1000.0 / 60.0).count();
    writeln!(output,
        "GALAXY_SUMMARY {phase} frames={} duration_seconds={:.6} mean_ms={:.3} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3} max_ms={:.3} over_16_667_ms={} over_budget_percent={:.2}",
        sorted.len(), sorted.iter().sum::<f64>() / 1000.0,
        sorted.iter().sum::<f64>() / sorted.len() as f64,
        percentile(0.5), percentile(0.95), percentile(0.99), sorted[sorted.len() - 1],
        missed, missed as f64 / sorted.len() as f64 * 100.0,
    ).unwrap();
    for (elapsed, ms) in samples {
        writeln!(output, "GALAXY_FRAME {phase} {elapsed:.6} {ms:.6}").unwrap();
    }
}
