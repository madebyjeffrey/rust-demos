//! A light, asymmetric garden workspace using the shared composition/frame library.

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
};
use bevy_composable_ui::prelude::*;

const PAGE: Color = Color::srgb(0.95, 0.94, 0.89);
const PAPER: Color = Color::srgb(0.99, 0.98, 0.95);
const INK: Color = Color::srgb(0.15, 0.25, 0.20);
const MUTED: Color = Color::srgb(0.43, 0.48, 0.42);
const GREEN: Color = Color::srgb(0.27, 0.43, 0.32);
const CLAY: Color = Color::srgb(0.69, 0.37, 0.25);
const EDGE: Color = Color::srgb(0.80, 0.82, 0.75);
const WASH: Color = Color::srgb(0.87, 0.90, 0.82);

#[derive(Clone, Debug, PartialEq)]
struct Plant {
    id: u64,
    visits: u32,
}

#[derive(Resource)]
struct GardenState {
    compact: bool,
    fine: bool,
    guides: bool,
    watered: u32,
    plants: Vec<Plant>,
    next_id: u64,
}

impl Default for GardenState {
    fn default() -> Self {
        Self {
            compact: false,
            fine: false,
            guides: false,
            watered: 0,
            plants: (1..=3).map(|id| Plant { id, visits: 0 }).collect(),
            next_id: 4,
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Action {
    Fine,
    Guides,
    Water,
    Reverse,
    Add,
    Remove,
}

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Sidebar;
#[derive(Component)]
struct GardenFrame;
#[derive(Component)]
struct Plot(usize);
#[derive(Component)]
struct WaterCount;
#[derive(Component)]
struct Footer;
#[derive(Component)]
struct GrowthMeters;
#[derive(Component)]
struct PlantControl(u64);

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(PAGE))
        .init_resource::<GardenState>()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Field Notes - composable UI garden".into(),
                    resolution: (1100, 840).into(),
                    resize_constraints: bevy::window::WindowResizeConstraints {
                        min_width: 400.,
                        min_height: 640.,
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
        .add_systems(Update, plant_focus)
        .add_systems(Startup, setup)
        .add_systems(Update, (keyboard, adapt, scroll, button_styles).chain());
    capture::install(&mut app);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let mut workspace = column((
        header(),
        plots(),
        Binding::read(|state: &GardenState| {
            (state.compact, state.fine, state.watered, state.plants.len())
        })
        .view(|(compact, fine, watered, count)| {
            text(
                format!(
                    "{} / {} / {watered} watering sessions / {count} plants",
                    if *compact {
                        "STACKED PLOTS"
                    } else {
                        "GARDEN GRID"
                    },
                    if *fine {
                        "FINE LINES"
                    } else {
                        "CURVED BRACKET"
                    }
                ),
                10.,
                MUTED,
            )
            .tagged(Footer)
        }),
    ))
    .gap(18.)
    .grow(1.);
    workspace.node.min_width = px(0);
    let mut root = row((sidebar(), workspace)).gap(32.).pad(28.);
    root.node.width = percent(100);
    root.node.height = percent(100);
    root.node.overflow = Overflow::scroll_y();
    root.tagged((Root, TabGroup::new(0), ScrollPosition::default()))
        .mount(&mut commands);
}

fn sidebar() -> impl View {
    let mut view = column((
        text("field / notes", 20., GREEN),
        text("A SMALL GARDEN\nWITH ROOM TO GROW", 10., MUTED),
        text("THIS WEEK", 10., CLAY)
            .padding(12.)
            .background(WASH)
            .tagged(SharedFrame::new([Rail::new(
                [
                    Anchor::frame(0., 1.).offset(8., -3.),
                    Anchor::frame(1., 1.).offset(-8., -3.),
                ],
                1.5,
                CLAY,
            )])),
        column((
            Binding::read(|state: &GardenState| state.plants.len())
                .view(|count| text(format!("{count:02}  plant varieties"), 12., INK)),
            text("18  new leaves", 12., INK),
            text("03  quiet corners", 12., INK),
        ))
        .gap(18.),
        text("Grow at your\nown pace.", 16., GREEN),
    ))
    .gap(28.);
    view.node.width = px(180);
    view.node.flex_shrink = 0.;
    view.tagged(Sidebar)
}

fn header() -> impl View {
    let mut controls = row((
        button("Fine lines  [T]", Action::Fine),
        button("Guides  [G]", Action::Guides),
        button("Reverse [R]", Action::Reverse),
        button("Add [N]", Action::Add),
        button("Remove [D]", Action::Remove),
    ))
    .gap(8.);
    controls.node.flex_wrap = FlexWrap::Wrap;
    let mut view = column((
        text("THE WINDOWSILL / BED 04", 10., CLAY),
        text("Room to grow.", 34., INK),
        controls,
    ))
    .gap(10.);
    view.node.flex_shrink = 0.;
    view
}

fn card<C: Views>(children: C) -> Element<C> {
    let mut view = column(children)
        .gap(14.)
        .pad(22.)
        .fill(PAPER)
        .border(1., EDGE, 3.);
    view.node.overflow = Overflow::clip();
    view
}

fn plots() -> impl View {
    let mut lead = card((
        text("01 / GROWTH", 10., GREEN),
        text("Slow mornings.\nSteady growth.", 25., INK),
        row((
            text("18", 76., GREEN),
            text("NEW LEAVES\nTHIS MONTH", 10., MUTED),
        ))
        .gap(10.),
        column((
            meter("Daylight", 0.79),
            meter("Humidity", 0.62),
            meter("Soil health", 0.88),
        ))
        .gap(20.)
        .tagged(GrowthMeters),
    ));
    lead.node.grid_row = GridPlacement::span(2);
    let water = card((
        text("02 / WATERING", 10., CLAY),
        text("A little care.", 22., INK),
        Binding::read(|state: &GardenState| state.watered).view(|watered| {
            text(format!("{watered} watering sessions"), 13., MUTED).tagged(WaterCount)
        }),
        button("Water plants  +", Action::Water),
    ));
    let journal = card((
        text("03 / FIELD JOURNAL", 10., GREEN),
        text("Today's observations", 18., INK),
        plants(),
        text("R reverses with motion. N adds; D removes.", 10., CLAY),
    ));
    let mut frame = grid((
        lead.tagged(Plot(0)),
        water.tagged(Plot(1)),
        journal.tagged(Plot(2)),
    ))
    .gap(36.)
    .grow(1.);
    frame.node.grid_template_columns = vec![GridTrack::flex(1.25), GridTrack::flex(1.)];
    frame.node.grid_template_rows = RepeatedGridTrack::flex(2, 1.);
    frame.node.padding = UiRect {
        right: px(48),
        bottom: px(48),
        ..default()
    };
    frame.tagged((GardenFrame, SharedFrame::new([])))
}

fn meter(label: &'static str, amount: f32) -> impl View {
    let mut fill = column(()).fill(GREEN);
    fill.node.width = percent(amount * 100.);
    fill.node.height = px(7);
    let mut track = row((fill,)).fill(WASH);
    track.node.width = percent(100);
    track.node.height = px(7);
    column((text(label, 11., MUTED), track)).gap(8.)
}

fn button(label: &'static str, action: Action) -> impl View {
    let mut view = row((text(label, 11., GREEN),))
        .pad(12.)
        .fill(PAPER)
        .border(1., GREEN, 3.);
    view.node.min_height = px(44);
    view.node.align_items = AlignItems::Center;
    view.node.justify_content = JustifyContent::Center;
    view.node.flex_shrink = 0.;
    view.tagged((
        Button,
        action,
        Hovered::default(),
        TabIndex(0),
        Pickable::default(),
    ))
}

fn activate(event: On<Activate>, actions: Query<&Action>, mut state: ResMut<GardenState>) {
    if let Ok(action) = actions.get(event.entity) {
        apply(*action, &mut state);
    }
}

fn apply(action: Action, state: &mut GardenState) {
    match action {
        Action::Fine => state.fine = !state.fine,
        Action::Guides => state.guides = !state.guides,
        Action::Water => state.watered += 1,
        Action::Reverse => state.plants.reverse(),
        Action::Add => {
            if state.plants.len() < 5 {
                let id = state.next_id;
                state.next_id += 1;
                state.plants.push(Plant { id, visits: 0 });
            }
        }
        Action::Remove => {
            state.plants.pop();
        }
    }
}

fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    mut state: ResMut<GardenState>,
) {
    for (key, action) in [
        (KeyCode::KeyR, Action::Reverse),
        (KeyCode::KeyN, Action::Add),
        (KeyCode::KeyD, Action::Remove),
    ] {
        if keys.just_pressed(key) {
            apply(action, &mut state);
        }
    }
    if keys.just_pressed(KeyCode::KeyT) {
        apply(Action::Fine, &mut state);
    }
    if keys.just_pressed(KeyCode::KeyG) {
        apply(Action::Guides, &mut state);
    }
    if keys.just_pressed(KeyCode::Space) && focus.get().is_none() {
        apply(Action::Water, &mut state);
    }
}

#[expect(
    clippy::type_complexity,
    reason = "Disjoint Bevy queries separate responsive nodes from the owning frame."
)]
fn adapt(
    window: Single<&Window>,
    mut state: ResMut<GardenState>,
    mut nodes: Query<
        (
            &mut Node,
            Option<&Sidebar>,
            Option<&Plot>,
            Option<&GrowthMeters>,
        ),
        Without<GardenFrame>,
    >,
    mut frame: Single<(&mut Node, &mut SharedFrame), With<GardenFrame>>,
    plots: Query<(Entity, &Plot)>,
) {
    let compact = window.width() < 900.;
    if state.compact != compact {
        state.compact = compact;
    }
    for (mut node, sidebar, plot, meters) in &mut nodes {
        if sidebar.is_some() {
            let display = if compact {
                Display::None
            } else {
                Display::Flex
            };
            if node.display != display {
                node.display = display;
            }
        }
        if let Some(plot) = plot {
            let span = GridPlacement::span(if !compact && plot.0 == 0 { 2 } else { 1 });
            if node.grid_row != span {
                node.grid_row = span;
            }
            let padding = UiRect::all(px(if compact { 14. } else { 22. }));
            let gap = px(if compact { 10. } else { 14. });
            if node.padding != padding {
                node.padding = padding;
            }
            if node.row_gap != gap {
                node.row_gap = gap;
            }
        }
        if meters.is_some() {
            let display = if compact {
                Display::None
            } else {
                Display::Flex
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
    let (node, shared) = &mut *frame;
    let columns = if compact {
        RepeatedGridTrack::flex(1, 1.)
    } else {
        vec![GridTrack::flex(1.25), GridTrack::flex(1.)]
    };
    let rows = RepeatedGridTrack::flex(if compact { 3 } else { 2 }, 1.);
    if node.grid_template_columns != columns {
        node.grid_template_columns = columns;
    }
    if node.grid_template_rows != rows {
        node.grid_template_rows = rows;
    }
    let minimum = px(if compact { 780. } else { 580. });
    if node.min_height != minimum {
        node.min_height = minimum;
    }
    let mut entities = [Entity::PLACEHOLDER; 3];
    for (entity, plot) in &plots {
        entities[plot.0] = entity;
    }
    let split_x = AxisAnchor::Between((entities[0], 1.), (entities[1], 0.));
    let gap_y = AxisAnchor::Between((entities[1], 1.), (entities[2], 0.));
    let divider = if compact {
        [
            Anchor::at(AxisAnchor::Frame(0.), gap_y).offset(12., 0.),
            Anchor::at(AxisAnchor::Frame(1.), gap_y).offset(-18., 0.),
        ]
        .to_vec()
    } else {
        [
            Anchor::at(split_x, AxisAnchor::Frame(0.)).offset(0., 12.),
            Anchor::at(split_x, gap_y),
            Anchor::at(AxisAnchor::Frame(1.), gap_y).offset(-18., 0.),
        ]
        .to_vec()
    };
    let mut next = SharedFrame::new([
        Rail::new(
            [
                Anchor::frame(0., 1.).offset(12., -16.),
                Anchor::frame(1., 1.).offset(-18., -16.),
                Anchor::frame(1., 0.).offset(-18., 12.),
            ],
            if state.fine { 1.5 } else { 10. },
            GREEN,
        )
        .rounded(28.),
        Rail::new(divider, if state.fine { 1.5 } else { 3. }, CLAY).rounded(18.),
    ]);
    next.guides = state.guides;
    if **shared != next {
        **shared = next;
    }
}

fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    mut root: Single<(&mut ScrollPosition, &ComputedNode), With<Root>>,
) {
    let max = ((root.1.content_size().y - root.1.size().y) * root.1.inverse_scale_factor()).max(0.);
    let mut y = root.0.y;
    for event in wheel.read() {
        y -= event.y
            * if event.unit == MouseScrollUnit::Line {
                32.
            } else {
                1.
            };
    }
    if keys.just_pressed(KeyCode::PageDown) {
        y += 320.;
    }
    if keys.just_pressed(KeyCode::PageUp) {
        y -= 320.;
    }
    if keys.just_pressed(KeyCode::Home) {
        y = 0.;
    }
    if keys.just_pressed(KeyCode::End) {
        y = max;
    }
    y = y.clamp(0., max);
    if root.0.y != y {
        root.0.y = y;
    }
}

#[expect(
    clippy::type_complexity,
    reason = "Native control state and app-owned style are queried together."
)]
fn button_styles(
    state: Res<GardenState>,
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
            Action::Fine => state.fine,
            Action::Guides => state.guides,
            Action::Water | Action::Reverse | Action::Add | Action::Remove => false,
        };
        let color = if selected || pressed || hovered.get() {
            WASH
        } else {
            PAPER
        };
        if background.0 != color {
            background.0 = color;
        }
        let color = if focus.get() == Some(entity) {
            CLAY
        } else {
            GREEN
        };
        if *border != BorderColor::all(color) {
            *border = BorderColor::all(color);
        }
    }
}

fn plants() -> impl View {
    let binding = Binding::resource(
        |state: &GardenState| state.plants.clone(),
        |state: &mut GardenState, plants| state.plants = plants,
    );
    let writer = binding.clone();
    binding.view(move |plants| {
        let mut chips = row(plants
            .iter()
            .map(|plant| {
                let id = plant.id;
                let writer = writer.clone();
                let name = ["Basil", "Mint", "Fern", "Thyme", "Sage"][((id - 1) % 5) as usize];
                let mut chip = row((text(format!("{name} {}", plant.visits), 11., GREEN),))
                    .pad(10.)
                    .fill(WASH)
                    .border(1., EDGE, 3.);
                chip.node.min_height = px(44);
                chip.node.align_items = AlignItems::Center;
                chip.tagged((
                    Button,
                    PlantControl(id),
                    Hovered::default(),
                    TabIndex(0),
                    Pickable::default(),
                ))
                .on_activate(move |world| {
                    writer
                        .update(world, |plants| {
                            if let Some(plant) = plants.iter_mut().find(|plant| plant.id == id) {
                                plant.visits += 1;
                            }
                        })
                        .expect("plant binding")
                })
                .key(id)
                .transition(ViewTransition {
                    duration: std::time::Duration::from_millis(520),
                    enter_offset: Vec2::new(18., 0.),
                    exit_offset: Vec2::new(-18., 0.),
                    edge_scale: 0.75,
                })
            })
            .collect::<Vec<_>>())
        .gap(6.);
        chips.node.flex_wrap = FlexWrap::Wrap;
        chips
    })
}

fn plant_focus(
    mut commands: Commands,
    focus: Res<InputFocus>,
    plants: Query<(Entity, Option<&Outline>), With<PlantControl>>,
) {
    for (entity, outline) in &plants {
        if focus.get() == Some(entity) && outline.is_none() {
            commands.entity(entity).insert(Outline {
                width: px(2),
                offset: px(2),
                color: CLAY,
            });
        } else if focus.get() != Some(entity) && outline.is_some() {
            commands.entity(entity).remove::<Outline>();
        }
    }
}
