use crate::{Action, GardenState, PlantControl, Plot, Root, WaterCount};
use bevy::{
    app::AppExit,
    camera::RenderTarget,
    ecs::system::SystemParam,
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
    ui_widgets::Activate,
};
use bevy_composable_ui::{
    ExitingView, FrameDiagnostic, FrameRegionsSystems, TransitionPhase, TransitionStatus,
    ViewDiagnostic, ViewTransitionSystems,
};
use std::path::PathBuf;

#[derive(Resource)]
struct Capture {
    dir: PathBuf,
    step: usize,
    configured: bool,
    pending: bool,
    ready: f64,
    identities: Vec<Entity>,
    keyed: Vec<(u64, Entity)>,
    motion_seen: [bool; 3],
    motion_pending: usize,
}
const NAMES: [&str; 4] = [
    "01-garden-grid",
    "02-garden-fine",
    "03-garden-compact",
    "04-garden-restored",
];

pub fn install(app: &mut App) {
    let args: Vec<_> = std::env::args().collect();
    if let Some(pair) = args.windows(2).find(|pair| pair[0] == "--capture-dir") {
        let dir = PathBuf::from(&pair[1]);
        std::fs::create_dir_all(&dir).expect("create capture directory");
        app.insert_resource(Capture {
            dir,
            step: 0,
            configured: false,
            pending: false,
            ready: 0.,
            identities: vec![],
            keyed: vec![],
            motion_seen: [false; 3],
            motion_pending: 0,
        })
        .add_systems(Update, capture)
        .add_systems(Startup, capture_target.after(crate::setup))
        .add_systems(
            PostUpdate,
            capture_motion
                .after(ViewTransitionSystems)
                .after(FrameRegionsSystems),
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Replay verifies real controls, geometry, window and retained state together."
)]
fn capture(
    mut commands: Commands,
    time: Res<Time>,
    mut capture: ResMut<Capture>,
    mut window: Single<&mut Window>,
    actions: Query<(Entity, &Action)>,
    plots: Query<(Entity, &Plot)>,
    state: Res<GardenState>,
    frames: Query<&FrameDiagnostic>,
    mut root: Single<&mut ScrollPosition, With<Root>>,
    keyed: Query<(Entity, &PlantControl, &ChildOf)>,
    children: Query<&Children>,
    texts: Query<&Text>,
    count: Single<&Text, With<WaterCount>>,
    bindings: Query<&ViewDiagnostic>,
    mut focus: ResMut<InputFocus>,
    mut target: CaptureTarget,
) {
    if capture.pending {
        return;
    }
    if !capture.configured {
        let (w, h) = if capture.step == 2 {
            (520., 1160.)
        } else {
            (1100., 840.)
        };
        window.resolution.set(w, h);
        target.resize(w as u32, h as u32);
        root.y = 0.;
        if capture.step == 0 {
            let entity = keyed.iter().find(|(_, item, _)| item.0 == 1).unwrap().0;
            focus.set(entity, FocusCause::Navigated);
        }
        if capture.step == 2 {
            let entity = keyed.iter().find(|(_, item, _)| item.0 == 2).unwrap().0;
            commands.trigger(Activate { entity });
        }
        for (entity, action) in &actions {
            if (*action == Action::Reverse && capture.step == 1)
                || (*action == Action::Add && capture.step == 2)
                || (*action == Action::Remove && capture.step == 3)
            {
                commands.trigger(Activate { entity });
            }
            if *action == Action::Fine && (capture.step == 1 || capture.step == 3) {
                commands.trigger(Activate { entity });
            }
            if *action == Action::Water && capture.step == 1 {
                for _ in 0..2 {
                    commands.trigger(Activate { entity });
                }
            }
        }
        capture.configured = true;
        capture.ready = time.elapsed_secs_f64() + 2.;
        return;
    }
    if time.elapsed_secs_f64() < capture.ready {
        return;
    }
    let mut ids: Vec<_> = plots.iter().map(|(e, plot)| (plot.0, e)).collect();
    ids.sort_by_key(|(index, _)| *index);
    let ids: Vec<_> = ids.into_iter().map(|(_, e)| e).collect();
    if capture.identities.is_empty() {
        capture.identities = ids.clone();
    }
    assert_eq!(
        capture.identities, ids,
        "plot identities survive CSS Grid reflow"
    );
    assert_eq!(state.watered, if capture.step == 0 { 0 } else { 2 });
    assert_eq!(state.compact, capture.step == 2);
    assert_eq!(
        frames.iter().count(),
        2,
        "independent main and sidebar frame owners"
    );
    assert!(
        frames.iter().all(|status| status.0.is_none()),
        "frame anchors resolve in every layout"
    );
    let current: Vec<_> = keyed
        .iter()
        .map(|(entity, item, _)| (item.0, entity))
        .collect();
    if capture.keyed.is_empty() {
        capture.keyed = current.clone();
    }
    for identity in &capture.keyed {
        assert!(current.contains(identity), "keyed plant entity is retained");
    }
    let selected = current.iter().find(|(key, _)| *key == 1).unwrap().1;
    assert_eq!(
        focus.get(),
        Some(selected),
        "plant focus survives reactive reorder, insertion and resize"
    );
    let parent = keyed.get(selected).unwrap().2.parent();
    let order: Vec<_> = children
        .get(parent)
        .unwrap()
        .iter()
        .map(|e| keyed.get(e).unwrap().1.0)
        .collect();
    assert_eq!(
        order,
        state
            .plants
            .iter()
            .map(|plant| plant.id)
            .collect::<Vec<_>>()
    );
    for (entity, item, _) in &keyed {
        let plant = state
            .plants
            .iter()
            .find(|plant| plant.id == item.0)
            .unwrap();
        let name = ["Basil", "Mint", "Fern", "Thyme", "Sage"][((plant.id - 1) % 5) as usize];
        assert_eq!(
            texts.get(children.get(entity).unwrap()[0]).unwrap().0,
            format!("{name} {}", plant.visits)
        );
    }
    assert_eq!(
        state
            .plants
            .iter()
            .find(|plant| plant.id == 2)
            .unwrap()
            .visits,
        if capture.step >= 2 { 1 } else { 0 }
    );
    assert_eq!(count.0, format!("{} watering sessions", state.watered));
    assert!(bindings.iter().all(|status| status.0.is_none()));
    if capture.step == NAMES.len() - 1 {
        assert!(
            capture.motion_seen.iter().all(|seen| *seen),
            "all transition phases rendered"
        );
        assert_eq!(capture.motion_pending, 0, "motion screenshots completed");
    }
    let name = NAMES[capture.step];
    std::fs::write(capture.dir.join(format!("{name}.txt")), format!("{name}\ncompact: {}\nfine: {}\nwatering sessions: {}\nretained plots: {:?}\nframe diagnostics: clear\n", state.compact, state.fine, state.watered, ids)).expect("write report");
    std::fs::write(capture.dir.join(format!("{name}-bindings.txt")),format!("keyed order: {order:?}\nretained plants: {current:?}\nfocused plant: {selected:?}\nbinding diagnostics: clear\n")).unwrap();
    let path = capture.dir.join(format!("{name}.png"));
    capture.pending = true;
    commands
        .spawn(Screenshot::image(target.image.0.clone()))
        .observe(
            move |image: On<ScreenshotCaptured>,
                  mut capture: ResMut<Capture>,
                  mut exit: MessageWriter<AppExit>| {
                let image = image
                    .image
                    .clone()
                    .try_into_dynamic()
                    .expect("8-bit screenshot");
                assert!(
                    image
                        .to_rgba8()
                        .pixels()
                        .any(|p| p[0] != 0 || p[1] != 0 || p[2] != 0),
                    "blank GPU capture"
                );
                image.save(&path).expect("save screenshot");
                capture.step += 1;
                if capture.step == NAMES.len() {
                    exit.write(AppExit::Success);
                } else {
                    capture.pending = false;
                    capture.configured = false;
                }
            },
        );
}

// Offscreen captures exercise the same camera and UI without depending on a native surface.
#[derive(Resource)]
struct CaptureImage(Handle<Image>);
#[derive(SystemParam)]
struct CaptureTarget<'w> {
    image: Res<'w, CaptureImage>,
    images: ResMut<'w, Assets<Image>>,
}
impl CaptureTarget<'_> {
    fn resize(&mut self, width: u32, height: u32) {
        let mut image = self.images.get_mut(&self.image.0).unwrap();
        image.resize(Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        });
    }
}
fn capture_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    camera: Single<Entity, With<Camera2d>>,
    root: Single<Entity, With<Root>>,
) {
    let image = images.add(Image::new_target_texture(
        1100,
        840,
        TextureFormat::Bgra8UnormSrgb,
        None,
    ));
    commands
        .entity(*camera)
        .insert((RenderTarget::Image(image.clone().into()), IsDefaultUiCamera));
    commands.entity(*root).insert(UiTargetCamera(*camera));
    commands.insert_resource(CaptureImage(image));
}

// Inspect a transition after layout, while its entities are actually in motion.
fn capture_motion(
    mut commands: Commands,
    mut capture: ResMut<Capture>,
    nodes: Query<(Entity, &PlantControl, &TransitionStatus, Has<ExitingView>)>,
    focus: Res<InputFocus>,
    image: Res<CaptureImage>,
) {
    let step = capture.step;
    let (index, name, expected, key) = match step {
        1 => (0, "motion-reorder", TransitionPhase::Moving, 1),
        2 => (1, "motion-insert", TransitionPhase::Entering, 4),
        3 => (2, "motion-remove", TransitionPhase::Exiting, 4),
        _ => return,
    };
    if capture.motion_seen[index] {
        return;
    }
    let Some((entity, _, status, exiting)) = nodes.iter().find(|(_, item, status, _)| {
        item.0 == key && status.phase == expected && (0.3..0.8).contains(&status.progress)
    }) else {
        return;
    };
    assert_eq!(exiting, expected == TransitionPhase::Exiting);
    for (key, retained) in &capture.keyed {
        let (current, _, _, exiting) = nodes.iter().find(|(_, item, _, _)| item.0 == *key).unwrap();
        assert_eq!(current, *retained, "identity is retained during motion");
        assert!(!exiting);
        if *key == 1 {
            assert_eq!(
                focus.get(),
                Some(current),
                "focus is retained during motion"
            );
        }
    }
    let path = capture.dir.join(format!("{name}.png"));
    std::fs::write(capture.dir.join(format!("{name}.txt")),
        format!("phase: {:?}\nprogress: {:.3}\nentity: {entity:?}\nfocus: {:?}\nidentity/focus during motion: retained\n", status.phase, status.progress, focus.get())).unwrap();
    capture.motion_seen[index] = true;
    capture.motion_pending += 1;
    commands.spawn(Screenshot::image(image.0.clone())).observe(
        move |image: On<ScreenshotCaptured>, mut capture: ResMut<Capture>| {
            let image = image.image.clone().try_into_dynamic().unwrap();
            assert!(
                image
                    .to_rgba8()
                    .pixels()
                    .any(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
            );
            image.save(&path).unwrap();
            capture.motion_pending -= 1;
        },
    );
}
