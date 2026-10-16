//! Opt-in release acceptance telemetry. It has no product UI and is inactive by default.
use std::time::{Duration, Instant};

use bevy::{app::AppExit, prelude::*, window::PrimaryWindow};

#[derive(Resource)]
struct FrameBaseline {
    started: Instant,
    first_frame: Option<Instant>,
    previous: Instant,
    warmup: Vec<(f64, f64)>,
    steady: Vec<(f64, f64)>,
    complete: bool,
}

pub(super) fn register(app: &mut App) {
    if std::env::var("GALAXY_FRAME_BASELINE").as_deref() != Ok("1") {
        return;
    }
    let now = Instant::now();
    app.insert_resource(FrameBaseline {
        started: now,
        first_frame: None,
        previous: now,
        warmup: Vec::new(),
        steady: Vec::new(),
        complete: false,
    })
    .add_systems(Last, capture_frame);
}

fn capture_frame(
    mut baseline: ResMut<FrameBaseline>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
) {
    if baseline.complete {
        return;
    }
    let now = Instant::now();
    let Some(first_frame) = baseline.first_frame else {
        println!(
            "GALAXY_STARTUP first_update_ms={:.3}",
            now.duration_since(baseline.started).as_secs_f64() * 1000.0
        );
        if let Ok(window) = window.single() {
            println!(
                "GALAXY_CONFIG physical_width={} physical_height={} scale={} present_mode={:?}",
                window.physical_width(),
                window.physical_height(),
                window.scale_factor(),
                window.present_mode
            );
        }
        baseline.first_frame = Some(now);
        baseline.previous = now;
        return;
    };
    let elapsed = now.duration_since(first_frame);
    let ms = now.duration_since(baseline.previous).as_secs_f64() * 1000.0;
    baseline.previous = now;
    let sample = (elapsed.as_secs_f64(), ms);
    if elapsed < Duration::from_secs(10) {
        baseline.warmup.push(sample);
    } else {
        baseline.steady.push(sample);
    }
    if elapsed >= Duration::from_secs(70) {
        // Printing during capture would add console transport cost to the next frame.
        report("warmup", &baseline.warmup);
        report("steady", &baseline.steady);
        baseline.complete = true;
        println!("GALAXY_COMPLETE");
        // winit intentionally ignores application exit on iOS. The host baseline
        // command stops its console-attached process after receiving this marker.
        if !cfg!(target_os = "ios") {
            exit.write(AppExit::Success);
        }
    }
}

fn report(phase: &str, samples: &[(f64, f64)]) {
    let mut sorted = samples.iter().map(|(_, ms)| *ms).collect::<Vec<_>>();
    sorted.sort_by(f64::total_cmp);
    if sorted.is_empty() {
        return;
    }
    let percentile = |p: f64| sorted[((sorted.len() - 1) as f64 * p).ceil() as usize];
    let missed = sorted.iter().filter(|ms| **ms > 1000.0 / 60.0).count();
    println!(
        "GALAXY_SUMMARY {phase} frames={} mean_ms={:.3} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3} max_ms={:.3} over_16_667_ms={} over_budget_percent={:.2}",
        sorted.len(),
        sorted.iter().sum::<f64>() / sorted.len() as f64,
        percentile(0.5),
        percentile(0.95),
        percentile(0.99),
        sorted[sorted.len() - 1],
        missed,
        missed as f64 / sorted.len() as f64 * 100.0
    );
    for (elapsed, ms) in samples {
        println!("GALAXY_FRAME {phase} {elapsed:.6} {ms:.6}");
    }
}
