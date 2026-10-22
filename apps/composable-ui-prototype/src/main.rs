//! Native, in-memory prototype of composable views and shared frame regions.

mod capture;

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    input_focus::{
        InputFocus,
        tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    },
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{Activate, Button},
    window::WindowResolution,
};
use bevy_composable_ui::{prelude::*, views};

const INK: Color = Color::srgb(0.88, 0.93, 0.94);
const QUIET: Color = Color::srgb(0.51, 0.62, 0.67);
const MINT: Color = Color::srgb(0.37, 0.83, 0.77);
const AMBER: Color = Color::srgb(0.98, 0.67, 0.38);
const PANEL: Color = Color::srgb(0.055, 0.092, 0.12);
const EDGE: Color = Color::srgb(0.15, 0.22, 0.26);
const BREAKPOINT: f32 = 820.;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Profile {
    Thin,
    #[default]
    Mixed,
    Thick,
}

#[derive(Clone, Debug, PartialEq)]
struct Reading {
    id: u64,
    value: u32,
}

#[derive(Resource)]
struct DemoState {
    profile: Profile,
    force_stack: bool,
    compact: bool,
    guides: bool,
    translucent: bool,
    samples: u32,
    readings: Vec<Reading>,
    next_id: u64,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            profile: Profile::Mixed,
            force_stack: false,
            compact: false,
            guides: false,
            translucent: false,
            samples: 0,
            readings: (1..=3).map(|id| Reading { id, value: 0 }).collect(),
            next_id: 4,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Profile(Profile),
    Stack,
    Guides,
    Alpha,
    Record,
    Reverse,
    Add,
    Remove,
}

#[derive(Component)]
struct FrameRoot;
#[derive(Component)]
struct DisplaySlot(usize);
#[derive(Component)]
struct MainSlots;
#[derive(Component)]
struct AppRoot;
#[derive(Component)]
struct Readout;
#[derive(Component)]
struct SampleCount;
#[derive(Component)]
struct ReadingControl(u64);
#[derive(Component)]
struct WideDetail;
#[derive(Component)]
struct MetricValue;

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.027, 0.047, 0.065)))
        .init_resource::<DemoState>()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Composable UI - shared frame prototype".into(),
                    resolution: WindowResolution::new(1120, 820),
                    resize_constraints: bevy::window::WindowResizeConstraints {
                        min_width: 380.,
                        min_height: 620.,
                        ..default()
                    },
                    ..default()
                }),
                ..default()
            }),
            TabNavigationPlugin,
            ComposableUiPlugin,
        ))
        .add_observer(activate)
        .add_systems(Update, reading_focus)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                shortcuts,
                responsive_layout,
                configure_frame,
                scroll_viewport,
                button_styles,
            )
                .chain(),
        );
    capture::install(&mut app);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let mut root = column((
        header(),
        shared_displays(),
        text("Resize to reflow  /  1-3 rail profiles  /  S stack  /  G guides  /  A alpha / R reverse / N add / D remove / Space record", 11., QUIET),
        Binding::read(|state: &DemoState| (state.compact, state.profile, state.samples)).view(|(compact,profile,samples)| {
            let (shell,divider)=profile.widths();
            text(format!("{} / {profile:?} / shell {shell} dp + divider {divider} dp / {samples} samples", if *compact { "STACKED" } else { "WIDE" }),11.,MINT).tagged(Readout)
        }),
    )).gap(16.).pad(28.);
    root.node.width = percent(100);
    root.node.height = percent(100);
    root.node.overflow = Overflow::scroll_y();
    root.tagged((AppRoot, TabGroup::new(0), ScrollPosition::default()))
        .mount(&mut commands);
}

fn header() -> impl View {
    let mut controls = row(vec![
        control("1  Thin", Action::Profile(Profile::Thin)).into_node(),
        control("2  Mixed", Action::Profile(Profile::Mixed)).into_node(),
        control("3  Thick", Action::Profile(Profile::Thick)).into_node(),
        control("Stack", Action::Stack).into_node(),
        control("Guides", Action::Guides).into_node(),
        control("50% alpha", Action::Alpha).into_node(),
        control("Reverse [R]", Action::Reverse).into_node(),
        control("Add [N]", Action::Add).into_node(),
        control("Remove [D]", Action::Remove).into_node(),
    ])
    .gap(8.);
    controls.node.flex_wrap = FlexWrap::Wrap;
    let mut header = column((
        text("COMPOSE / BEVY     -     NATIVE RUST PROTOTYPE", 11., MINT)
            .padding(4.)
            .background(PANEL),
        text("Shared space.", 38., INK),
        text(
            "Reusable views. Responsive slots. One continuous frame.",
            14.,
            QUIET,
        ),
        controls,
    ))
    .gap(10.);
    header.node.flex_shrink = 0.;
    header
}

fn shared_displays() -> impl View {
    let slots = row((
        telemetry().tagged(DisplaySlot(0)),
        diagnostics().tagged(DisplaySlot(1)),
    ))
    .gap(52.)
    .grow(1.)
    .tagged(MainSlots);
    let mut frame = column((slots, session().tagged(DisplaySlot(2))))
        .gap(48.)
        .grow(1.);
    // Reservation remains fixed for all profiles. Only paint thickness changes.
    frame.node.padding = UiRect {
        left: px(56),
        top: px(56),
        right: px(0),
        bottom: px(0),
    };
    frame.tagged((FrameRoot, SharedFrame::new([])))
}

impl Profile {
    fn widths(self) -> (f32, f32) {
        match self {
            Self::Thin => (1.5, 1.5),
            Self::Mixed => (24., 6.),
            Self::Thick => (24., 24.),
        }
    }
}

fn configure_frame(
    state: Res<DemoState>,
    slots: Query<(Entity, &DisplaySlot)>,
    mut frame: Single<&mut SharedFrame, With<FrameRoot>>,
) {
    let mut entities = [Entity::PLACEHOLDER; 3];
    for (entity, slot) in &slots {
        entities[slot.0] = entity;
    }
    let split_x = AxisAnchor::Between((entities[0], 1.), (entities[1], 0.));
    let gap_y = if state.compact {
        AxisAnchor::Between((entities[0], 1.), (entities[1], 0.))
    } else {
        AxisAnchor::Between((entities[1], 1.), (entities[2], 0.))
    };
    let divider = if state.compact {
        [
            Anchor::at(AxisAnchor::Frame(0.), gap_y).offset(18., -36.),
            Anchor::at(AxisAnchor::Frame(0.), gap_y).offset(18., 0.),
            Anchor::at(AxisAnchor::Frame(1.), gap_y).offset(-12., 0.),
        ]
    } else {
        [
            Anchor::at(split_x, AxisAnchor::Frame(0.)).offset(0., 18.),
            Anchor::at(split_x, gap_y),
            Anchor::at(AxisAnchor::Frame(1.), gap_y).offset(-12., 0.),
        ]
    };
    let (shell, width) = state.profile.widths();
    let mut next = SharedFrame::new([
        Rail::new(
            [
                Anchor::frame(0., 1.).offset(18., -12.),
                Anchor::frame(0., 0.).offset(18., 18.),
                Anchor::frame(1., 0.).offset(-12., 18.),
            ],
            shell,
            MINT,
        )
        .rounded(28.),
        Rail::new(divider, width, AMBER).rounded(22.),
    ]);
    next.guides = state.guides;
    next.opacity = if state.translucent { 0.5 } else { 1. };
    if **frame != next {
        **frame = next;
    }
}

fn display<C: views::Views>(children: C) -> views::Element<C> {
    let mut view = column(children)
        .gap(15.)
        .pad(22.)
        .grow(1.)
        .fill(PANEL)
        .border(1., EDGE, 10.);
    view.node.overflow = Overflow::clip();
    view
}

fn chart(samples: u32) -> impl View {
    let mut chart = row((0..28)
        .map(|i| {
            let mut bar = column(Vec::<views::Label>::new()).grow(1.).fill(MINT);
            bar.node.height = percent(bar_height(i, samples));
            bar.node.border_radius = BorderRadius::all(px(2));
            bar.key(i)
        })
        .collect::<Vec<_>>())
    .gap(4.)
    .grow(1.);
    chart.node.align_items = AlignItems::End;
    chart.node.min_height = px(24);
    chart
}

fn telemetry() -> impl View {
    let chart = Binding::read(|state: &DemoState| state.samples)
        .view(|samples| chart(*samples))
        .grow(1.);
    let mut metric = row((
        text("98.2%", 42., INK).tagged(MetricValue),
        text("CARRIER INTEGRITY", 10., QUIET),
    ))
    .gap(10.);
    metric.node.align_items = AlignItems::Center;
    display((
        text("01 / TELEMETRY", 11., MINT),
        metric,
        chart,
        readings(),
        text(
            "Tab to a sensor. R reverses without moving focus.",
            10.,
            QUIET,
        ),
    ))
}

fn diagnostics() -> impl View {
    display((
        text("02 / DIAGNOSTICS", 11., AMBER),
        text("One frame owner.", 25., INK),
        text(
            "The curved divider follows both displays. Their content never draws the shared border.",
            13.,
            QUIET,
        ),
        status_line("Layout", "Flex / intrinsic").tagged(WideDetail),
        status_line("Frame", "Analytic region union").tagged(WideDetail),
        status_line("Identity", "Retained across resize").tagged(WideDetail),
    ))
}

fn status_line(label: &'static str, value: &'static str) -> impl View {
    let mut line = row((text(label, 11., QUIET), text(value, 11., INK))).gap(12.);
    line.node.justify_content = JustifyContent::SpaceBetween;
    line
}

fn session() -> impl View {
    let mut strip = row((
        column((
            text("03 / SESSION", 10., AMBER),
            Binding::read(|state: &DemoState| state.samples).view(|samples| {
                text(format!("{samples} samples recorded"), 16., INK).tagged(SampleCount)
            }),
        ))
        .gap(5.),
        control("Record sample  +", Action::Record),
    ))
    .gap(12.)
    .pad(18.)
    .fill(PANEL)
    .border(1., EDGE, 10.);
    strip.node.justify_content = JustifyContent::SpaceBetween;
    strip.node.align_items = AlignItems::Center;
    strip.node.flex_shrink = 0.;
    strip
}

struct Control {
    label: &'static str,
    action: Action,
}

fn control(label: &'static str, action: Action) -> Control {
    Control { label, action }
}

impl View for Control {
    fn into_node(self) -> ViewNode {
        let mut view = row((text(self.label, 12., INK),))
            .pad(12.)
            .fill(PANEL)
            .border(1., EDGE, 7.);
        view.node.min_height = px(44);
        view.node.flex_shrink = 0.;
        view.node.align_items = AlignItems::Center;
        view.node.justify_content = JustifyContent::Center;
        view.tagged((
            Button,
            self.action,
            Hovered::default(),
            TabIndex(0),
            Pickable::default(),
        ))
        .into_node()
    }
}

fn readings() -> impl View {
    let binding = Binding::resource(
        |state: &DemoState| state.readings.clone(),
        |state: &mut DemoState, value| state.readings = value,
    );
    let writer = binding.clone();
    binding.view(move |items| {
        let mut chips = row(items
            .iter()
            .map(|item| {
                let id = item.id;
                let writer = writer.clone();
                let mut chip = row((text(format!("S{id} +{}", item.value), 10., MINT),))
                    .pad(10.)
                    .fill(PANEL)
                    .border(1., EDGE, 6.);
                chip.node.min_height = px(44);
                chip.node.align_items = AlignItems::Center;
                chip.tagged((
                    Button,
                    ReadingControl(id),
                    Hovered::default(),
                    TabIndex(0),
                    Pickable::default(),
                ))
                .on_activate(move |world| {
                    writer
                        .update(world, |items| {
                            if let Some(item) = items.iter_mut().find(|item| item.id == id) {
                                item.value += 1;
                            }
                        })
                        .expect("sensor binding")
                })
                .key(id)
            })
            .collect::<Vec<_>>())
        .gap(6.);
        chips.node.flex_wrap = FlexWrap::Wrap;
        chips
    })
}

fn activate(event: On<Activate>, actions: Query<&Action>, mut state: ResMut<DemoState>) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    apply_action(*action, &mut state);
}

fn apply_action(action: Action, state: &mut DemoState) {
    match action {
        Action::Profile(profile) => state.profile = profile,
        Action::Stack => state.force_stack = !state.force_stack,
        Action::Guides => state.guides = !state.guides,
        Action::Alpha => state.translucent = !state.translucent,
        Action::Record => state.samples += 1,
        Action::Reverse => state.readings.reverse(),
        Action::Add => {
            if state.readings.len() < 5 {
                let id = state.next_id;
                state.next_id += 1;
                state.readings.push(Reading { id, value: 0 });
            }
        }
        Action::Remove => {
            state.readings.pop();
        }
    }
    info!(?action, profile = ?state.profile, stacked = state.force_stack,
        guides = state.guides, alpha = state.translucent, samples = state.samples, "UI state");
}

fn shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    mut state: ResMut<DemoState>,
) {
    for (key, action) in [
        (KeyCode::Digit1, Action::Profile(Profile::Thin)),
        (KeyCode::Digit2, Action::Profile(Profile::Mixed)),
        (KeyCode::Digit3, Action::Profile(Profile::Thick)),
        (KeyCode::KeyS, Action::Stack),
        (KeyCode::KeyG, Action::Guides),
        (KeyCode::KeyA, Action::Alpha),
        (KeyCode::KeyR, Action::Reverse),
        (KeyCode::KeyN, Action::Add),
        (KeyCode::KeyD, Action::Remove),
    ] {
        if keys.just_pressed(key) {
            apply_action(action, &mut state);
        }
    }
    // Space/Enter on a focused button is handled by Bevy's headless widget.
    if keys.just_pressed(KeyCode::Space) && focus.get().is_none() {
        apply_action(Action::Record, &mut state);
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "Bevy system parameters declare disjoint mutable UI queries."
)]
fn responsive_layout(
    window: Single<&Window>,
    mut state: ResMut<DemoState>,
    mut slots: Single<&mut Node, With<MainSlots>>,
    mut root: Single<&mut Node, (With<AppRoot>, Without<MainSlots>)>,
    mut frame: Single<&mut Node, (With<FrameRoot>, Without<MainSlots>, Without<AppRoot>)>,
    mut displays: Query<
        (&DisplaySlot, &mut Node),
        (Without<MainSlots>, Without<AppRoot>, Without<FrameRoot>),
    >,
    mut details: Query<
        &mut Node,
        (
            With<WideDetail>,
            Without<DisplaySlot>,
            Without<MainSlots>,
            Without<AppRoot>,
            Without<FrameRoot>,
        ),
    >,
    mut metric: Single<&mut TextFont, With<MetricValue>>,
) {
    let compact = state.force_stack || window.width() < BREAKPOINT;
    if state.compact != compact {
        state.compact = compact;
        info!(
            compact,
            "Responsive layout plan changed; display entities retained"
        );
    }
    let direction = if compact {
        FlexDirection::Column
    } else {
        FlexDirection::Row
    };
    if slots.flex_direction != direction {
        slots.flex_direction = direction;
    }
    let padding = UiRect::all(px(if window.width() < 600. { 16. } else { 28. }));
    if root.padding != padding {
        root.padding = padding;
    }
    let minimum = px(if compact { 680. } else { 480. });
    if frame.min_height != minimum {
        frame.min_height = minimum;
    }
    for (slot, mut node) in &mut displays {
        if slot.0 == 2 {
            continue;
        }
        let padding = UiRect::all(px(if compact { 14. } else { 22. }));
        let gap = px(if compact { 8. } else { 15. });
        if node.padding != padding {
            node.padding = padding;
        }
        if node.row_gap != gap {
            node.row_gap = gap;
        }
    }
    for mut node in &mut details {
        let display = if compact {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
    }
    let size = FontSize::Px(if compact { 34. } else { 42. });
    if metric.font_size != size {
        metric.font_size = size;
    }
}

fn scroll_viewport(
    mut wheel: MessageReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    mut root: Single<(&mut ScrollPosition, &ComputedNode), With<AppRoot>>,
    focused_nodes: Query<(&ComputedNode, &UiGlobalTransform), With<Action>>,
    window: Single<&Window>,
) {
    let (scroll, node) = &mut *root;
    let max = ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.);
    let mut y = scroll.y;
    for event in wheel.read() {
        y -= event.y
            * if event.unit == MouseScrollUnit::Line {
                32.
            } else {
                1.
            };
    }
    if keys.just_pressed(KeyCode::PageDown) {
        y += window.height() * 0.7;
    }
    if keys.just_pressed(KeyCode::PageUp) {
        y -= window.height() * 0.7;
    }
    if keys.just_pressed(KeyCode::Home) {
        y = 0.;
    }
    if keys.just_pressed(KeyCode::End) {
        y = max;
    }
    if focus.is_changed()
        && let Some(entity) = focus.get()
        && let Ok((bounds, transform)) = focused_nodes.get(entity)
    {
        let top =
            (transform.affine().translation.y - bounds.size().y / 2.) * node.inverse_scale_factor();
        let bottom = top + bounds.size().y * node.inverse_scale_factor();
        if bottom > window.height() - 16. {
            y += bottom - window.height() + 16.;
        }
        if top < 16. {
            y += top - 16.;
        }
    }
    y = y.clamp(0., max);
    if scroll.y != y {
        scroll.y = y;
    }
}

fn bar_height(index: usize, samples: u32) -> f32 {
    18. + (((index * 17 + samples as usize * 7) % 29) as f32 / 29.) * 78.
}

#[expect(
    clippy::type_complexity,
    reason = "The query lists the button's native interaction and style components."
)]
fn button_styles(
    state: Res<DemoState>,
    focus: Res<InputFocus>,
    mut buttons: Query<(
        Entity,
        &Action,
        &Hovered,
        Has<Pressed>,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (entity, action, hovered, pressed, mut background, mut border) in &mut buttons {
        let selected = match action {
            Action::Profile(profile) => state.profile == *profile,
            Action::Stack => state.force_stack,
            Action::Guides => state.guides,
            Action::Alpha => state.translucent,
            Action::Record | Action::Reverse | Action::Add | Action::Remove => false,
        };
        let bg = if pressed {
            Color::srgb(0.16, 0.35, 0.34)
        } else if selected || hovered.get() {
            Color::srgb(0.09, 0.19, 0.21)
        } else {
            PANEL
        };
        let edge = if focus.get() == Some(entity) {
            INK
        } else if selected {
            MINT
        } else {
            EDGE
        };
        if background.0 != bg {
            background.0 = bg;
        }
        if *border != BorderColor::all(edge) {
            *border = BorderColor::all(edge);
        }
    }
}

fn reading_focus(
    mut commands: Commands,
    focus: Res<InputFocus>,
    readings: Query<(Entity, Option<&Outline>), With<ReadingControl>>,
) {
    for (entity, outline) in &readings {
        if focus.get() == Some(entity) && outline.is_none() {
            commands.entity(entity).insert(Outline {
                width: px(2),
                offset: px(2),
                color: AMBER,
            });
        } else if focus.get() != Some(entity) && outline.is_some() {
            commands.entity(entity).remove::<Outline>();
        }
    }
}
