use crate::{INK, Tone, Ui, builder::text_bundle};
use bevy::{
    input_focus::tab_navigation::TabGroup, picking::Pickable, prelude::*, window::PrimaryWindow,
};

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 800.0;
const CONTENT_LEFT: f32 = 224.0;
const SPINE_RIGHT: f32 = 184.0;
const INNER_RADIUS: f32 = CONTENT_LEFT - SPINE_RIGHT;
const TOP_BAR_BOTTOM: f32 = 50.0;
const BOTTOM_BAR_TOP: f32 = 758.0;

/// A framed desktop screen on a 1280×800 design canvas, fitted to the primary window.
/// Owns the curved frame, header, rail, content region, status line, and footer.
/// Content uses flex layout; applications never need the frame's internal coordinates.
pub struct Screen {
    title: String,
    heading: String,
    subtitle: String,
    identifier: String,
    rail: [String; 4],
}
/// Despawn `root` to remove the complete screen. Bind a component to `status` to
/// update its native `Text` in your own systems.
#[derive(Clone, Copy)]
pub struct ScreenEntities {
    pub root: Entity,
    pub status: Entity,
}
#[derive(Component)]
pub(crate) struct FittedScreen;

impl Screen {
    pub fn new(title: impl Into<String>, heading: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            heading: heading.into(),
            subtitle: String::new(),
            identifier: "01-001".into(),
            rail: ["01", "02", "03", "04"].map(String::from),
        }
    }
    pub fn subtitle(mut self, text: impl Into<String>) -> Self {
        self.subtitle = text.into();
        self
    }
    pub fn identifier(mut self, text: impl Into<String>) -> Self {
        self.identifier = text.into();
        self
    }
    pub fn rail(mut self, labels: [&str; 4]) -> Self {
        self.rail = labels.map(String::from);
        self
    }
    /// Build once during setup. The content closure receives a horizontal row;
    /// use `column(weight, ...)` for its top-level sections. Spawn your own camera.
    pub fn spawn(self, commands: &mut Commands, content: impl FnOnce(&mut Ui)) -> ScreenEntities {
        let root = commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(INK),
            ))
            .id();
        let canvas = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: px(WIDTH),
                    height: px(HEIGHT),
                    flex_shrink: 0.0,
                    ..default()
                },
                FittedScreen,
                TabGroup::default(),
            ))
            .id();
        commands.entity(root).add_child(canvas);
        commands.entity(canvas).with_children(|p| self.frame(p));
        let body = commands
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(28),
                ..absolute(CONTENT_LEFT, 190.0, WIDTH - CONTENT_LEFT - 28.0, 492.0)
            })
            .id();
        commands.entity(canvas).add_child(body);
        content(&mut Ui {
            commands,
            parent: body,
        });
        let status = commands
            .spawn((
                text_bundle("", 16.0, Tone::Peach.color()),
                absolute(CONTENT_LEFT, 721.0, WIDTH - CONTENT_LEFT - 28.0, 28.0),
            ))
            .id();
        commands.entity(canvas).add_child(status);
        ScreenEntities { root, status }
    }
    fn frame(&self, p: &mut ChildSpawnerCommands) {
        let orange = Tone::Orange.color();
        let lilac = Tone::Lilac.color();
        let peach = Tone::Peach.color();
        let blue = Tone::Blue.color();
        p.spawn((
            Node {
                border_radius: BorderRadius::top_left(px(76)),
                ..absolute(28.0, 26.0, CONTENT_LEFT - 28.0, 146.0)
            },
            BackgroundColor(orange),
            Pickable::IGNORE,
        ));
        // Both insets derive their tangent from the bar edge and content edge.
        // This prevents independent coordinates from producing a ledge at the join.
        p.spawn((
            Node {
                border_radius: BorderRadius::top_left(px(INNER_RADIUS)),
                ..absolute(SPINE_RIGHT, TOP_BAR_BOTTOM, INNER_RADIUS * 2.0, 124.0)
            },
            BackgroundColor(INK),
            Pickable::IGNORE,
        ));
        block(p, 217.0, 26.0, 240.0, TOP_BAR_BOTTOM - 26.0, orange);
        block(p, 465.0, 26.0, 118.0, 24.0, lilac);
        block(p, 591.0, 26.0, 58.0, 24.0, peach);
        caption(p, &self.title, (672.0, 22.0, 580.0), 26.0, orange);
        caption(p, &self.identifier, (60.0, 104.0, 120.0), 23.0, INK);
        caption(
            p,
            "LIBRARY COMPUTER ACCESS & RETRIEVAL SYSTEM",
            (CONTENT_LEFT, 72.0, 1028.0),
            13.0,
            crate::MUTED,
        );
        caption(p, &self.heading, (CONTENT_LEFT, 98.0, 1028.0), 34.0, peach);
        caption(
            p,
            &self.subtitle,
            (CONTENT_LEFT, 145.0, 1028.0),
            14.0,
            lilac,
        );
        for ((y, height, color), number) in [
            (180.0, 82.0, peach),
            (270.0, 142.0, lilac),
            (420.0, 104.0, blue),
            (532.0, 104.0, orange),
        ]
        .into_iter()
        .zip(&self.rail)
        {
            block(p, 28.0, y, SPINE_RIGHT - 28.0, height, color);
            caption(p, number, (58.0, y + height - 33.0, 120.0), 19.0, INK);
        }
        p.spawn((
            Node {
                border_radius: BorderRadius::bottom_left(px(70)),
                ..absolute(28.0, 644.0, CONTENT_LEFT - 28.0, 130.0)
            },
            BackgroundColor(lilac),
            Pickable::IGNORE,
        ));
        p.spawn((
            Node {
                border_radius: BorderRadius::bottom_left(px(INNER_RADIUS)),
                ..absolute(
                    SPINE_RIGHT,
                    640.0,
                    INNER_RADIUS * 2.0,
                    BOTTOM_BAR_TOP - 640.0,
                )
            },
            BackgroundColor(INK),
            Pickable::IGNORE,
        ));
        block(p, CONTENT_LEFT, 704.0, 1028.0, 2.0, orange);
        block(p, 217.0, BOTTOM_BAR_TOP, 491.0, 16.0, lilac);
        block(p, 716.0, BOTTOM_BAR_TOP, 98.0, 16.0, peach);
        caption(
            p,
            "TAB: FOCUS / ENTER: ACTIVATE",
            (834.0, 756.0, 418.0),
            14.0,
            crate::MUTED,
        );
    }
}

pub(crate) fn fit_screens(
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Option<Res<UiScale>>,
    mut screens: Query<&mut UiTransform, With<FittedScreen>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let global_scale = ui_scale.map_or(1.0, |s| s.0);
    if window.width() <= 0.0 || window.height() <= 0.0 || global_scale <= 0.0 {
        return;
    }
    let scale = (window.width() / WIDTH).min(window.height() / HEIGHT) / global_scale;
    for mut transform in &mut screens {
        transform.scale = Vec2::splat(scale);
        // UiTransform scales around the node's center. Position that center at
        // the viewport center explicitly, including when the canvas overflows.
        transform.translation = Val2::new(
            px((window.width() / global_scale - WIDTH) / 2.0),
            px((window.height() / global_scale - HEIGHT) / 2.0),
        );
    }
}
fn absolute(x: f32, y: f32, width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(x),
        top: px(y),
        width: px(width),
        height: px(height),
        ..default()
    }
}
fn block(p: &mut ChildSpawnerCommands, x: f32, y: f32, width: f32, height: f32, color: Color) {
    p.spawn((
        absolute(x, y, width, height),
        BackgroundColor(color),
        Pickable::IGNORE,
    ));
}
fn caption(
    p: &mut ChildSpawnerCommands,
    text: &str,
    (x, y, width): (f32, f32, f32),
    size: f32,
    color: Color,
) {
    p.spawn((
        text_bundle(text, size, color),
        absolute(x, y, width, size + 6.0),
    ));
}
