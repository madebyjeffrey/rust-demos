//! Optional real-render replay: resize one window, activate actual controls, capture PNGs.
//! Run with `--capture-dir output/ui-prototype` to reproduce the visual review.

use crate::{Action, AppRoot, DemoState, DisplaySlot, Profile, ReadingControl, SampleCount};
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
use bevy_composable_ui::{FrameDiagnostic, ViewDiagnostic};
use std::path::PathBuf;

#[derive(Resource)]
struct Replay {
    directory: PathBuf,
    step: usize,
    configured: bool,
    pending: bool,
    ready_at: f64,
    identities: Vec<Entity>,
    keyed: Vec<(u64, Entity)>,
}

const NAMES: [&str; 5] = [
    "01-wide-thin",
    "02-wide-thick-alpha",
    "03-compact-mixed",
    "04-short-scrolled",
    "05-wide-restored",
];

pub fn install(app: &mut App) {
    let args: Vec<_> = std::env::args().collect();
    let Some(pair) = args.windows(2).find(|pair| pair[0] == "--capture-dir") else {
        return;
    };
    let directory = PathBuf::from(&pair[1]);
    std::fs::create_dir_all(&directory).expect("create screenshot directory");
    app.insert_resource(Replay {
        directory,
        step: 0,
        configured: false,
        pending: false,
        ready_at: 0.,
        identities: vec![],
        keyed: vec![],
    })
    .add_systems(Update, replay)
    .add_systems(Startup, capture_target.after(crate::setup));
}

#[expect(
    clippy::too_many_arguments,
    reason = "The replay checks the real window, controls, layout, state and entity lifetimes together."
)]
fn replay(
    mut commands: Commands,
    time: Res<Time>,
    mut replay: ResMut<Replay>,
    mut window: Single<&mut Window>,
    actions: Query<(Entity, &Action)>,
    slots: Query<(Entity, &DisplaySlot, &ComputedNode)>,
    state: Res<DemoState>,
    mut root: Single<(&mut ScrollPosition, &ComputedNode), With<AppRoot>>,
    frames: Query<&FrameDiagnostic>,
    bindings: Query<&ViewDiagnostic>,
    keyed: Query<(Entity, &ReadingControl, &ChildOf)>,
    children: Query<&Children>,
    texts: Query<&Text>,
    count: Single<&Text, With<SampleCount>>,
    mut focus: ResMut<InputFocus>,
    mut target: CaptureTarget,
) {
    if replay.pending {
        return;
    }
    if !replay.configured {
        let step = replay.step;
        let (width, height) = match step {
            2 => (480., 1220.),
            3 => (480., 680.),
            _ => (1120., 920.),
        };
        window.resolution.set(width, height);
        target.resize(width as u32, height as u32);
        let profile = [
            Profile::Thin,
            Profile::Thick,
            Profile::Mixed,
            Profile::Mixed,
            Profile::Mixed,
        ][step];
        root.0.y = 0.;
        if step == 0 {
            let entity = keyed.iter().find(|(_, item, _)| item.0 == 2).unwrap().0;
            focus.set(entity, FocusCause::Navigated);
        }
        if step == 2 {
            let entity = keyed.iter().find(|(_, item, _)| item.0 == 2).unwrap().0;
            commands.trigger(Activate { entity });
        }
        for (entity, action) in &actions {
            if (*action == Action::Reverse && step == 1)
                || (*action == Action::Add && step == 2)
                || (*action == Action::Remove && step == 3)
            {
                commands.trigger(Activate { entity });
            }
            if *action == Action::Profile(profile)
                || (*action == Action::Alpha && (step == 1 || step == 2))
            {
                commands.trigger(Activate { entity });
            }
            if *action == Action::Record && step == 2 {
                for _ in 0..3 {
                    commands.trigger(Activate { entity });
                }
            }
        }
        replay.configured = true;
        replay.ready_at = time.elapsed_secs_f64() + 2.;
        return;
    }
    if time.elapsed_secs_f64() < replay.ready_at {
        if replay.step == 3 {
            root.0.y = ((root.1.content_size().y - root.1.size().y)
                * root.1.inverse_scale_factor())
            .max(0.);
        }
        return;
    }
    let mut identities: Vec<_> = slots
        .iter()
        .map(|(entity, slot, _)| (slot.0, entity))
        .collect();
    identities.sort_by_key(|(index, _)| *index);
    let identities: Vec<_> = identities.into_iter().map(|(_, entity)| entity).collect();
    if replay.identities.is_empty() {
        replay.identities = identities.clone();
    }
    assert_eq!(
        replay.identities, identities,
        "resize/profile changes must preserve display entities"
    );
    assert_eq!(
        state.compact,
        replay.step == 2 || replay.step == 3,
        "viewport drives the responsive plan"
    );
    if replay.step == 3 {
        assert!(root.0.y > 0., "short viewport can scroll to its controls");
    }
    assert_eq!(
        state.samples,
        if replay.step >= 2 { 3 } else { 0 },
        "button actions and state survive reflow"
    );
    let current: Vec<_> = keyed
        .iter()
        .map(|(entity, item, _)| (item.0, entity))
        .collect();
    if replay.keyed.is_empty() {
        replay.keyed = current.clone();
    }
    for identity in &replay.keyed {
        assert!(
            current.contains(identity),
            "keyed sensor entity is retained"
        );
    }
    let selected = current.iter().find(|(key, _)| *key == 2).unwrap().1;
    assert_eq!(
        focus.get(),
        Some(selected),
        "sensor focus survives reactive reorder, insertion and resize"
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
            .readings
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>()
    );
    for (entity, item, _) in &keyed {
        let value = state
            .readings
            .iter()
            .find(|reading| reading.id == item.0)
            .unwrap()
            .value;
        assert_eq!(
            texts.get(children.get(entity).unwrap()[0]).unwrap().0,
            format!("S{} +{value}", item.0)
        );
    }
    assert_eq!(
        state
            .readings
            .iter()
            .find(|item| item.id == 2)
            .unwrap()
            .value,
        if replay.step >= 2 { 1 } else { 0 }
    );
    assert_eq!(count.0, format!("{} samples recorded", state.samples));
    assert!(bindings.iter().all(|status| status.0.is_none()));
    let name = NAMES[replay.step];
    assert_eq!(frames.iter().count(), 1);
    assert!(
        frames.iter().all(|status| status.0.is_none()),
        "shared library resolves frame geometry"
    );
    let mut report = format!(
        "{name}\nprofile: {:?}\ncompact: {}\nsamples: {}\nretained display entities: {:?}\n",
        state.profile, state.compact, state.samples, identities
    );
    report.push_str(&format!("keyed order: {order:?}\nretained sensors: {current:?}\nfocused sensor: {selected:?}\nbinding diagnostics: clear\n"));
    for (_, slot, node) in &slots {
        report.push_str(&format!(
            "slot {} logical size: {:?}\n",
            slot.0,
            node.size() * node.inverse_scale_factor()
        ));
    }
    std::fs::write(replay.directory.join(format!("{name}.txt")), report)
        .expect("write replay report");
    let path = replay.directory.join(format!("{name}.png"));
    replay.pending = true;
    commands
        .spawn(Screenshot::image(target.image.0.clone()))
        .observe(
            move |image: On<ScreenshotCaptured>,
                  mut replay: ResMut<Replay>,
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
                info!(path = %path.display(), "Prototype capture complete");
                replay.step += 1;
                if replay.step == NAMES.len() {
                    exit.write(AppExit::Success);
                } else {
                    replay.configured = false;
                    replay.pending = false;
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
    root: Single<Entity, With<AppRoot>>,
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
