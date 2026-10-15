use bevy::{
    picking::Pickable,
    prelude::*,
    transform::{TransformPlugin, TransformSystems},
    window::PrimaryWindow,
};

use crate::{PlanetId, PlanetMarker};

const HOVER_LABEL_COLOR: Color = Color::srgb(1.0, 0.84, 0.62);
const HOVER_LABEL_BACKGROUND: Color = Color::srgba(0.025, 0.035, 0.09, 0.9);
const CARD_BACKGROUND: Color = Color::srgba(0.025, 0.035, 0.09, 0.9);
const CARD_PRIMARY: Color = Color::srgb(0.98, 0.83, 0.6);
const CARD_SECONDARY: Color = Color::srgb(0.72, 0.77, 0.85);
const CARD_QUIET: Color = Color::srgb(0.61, 0.68, 0.79);

/// A cursor ray submitted by the window adapter or a headless app test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetCursorRay {
    pub origin: Vec3,
    pub direction: Vec3,
}

/// Narrow input seam shared by the window adapter and headless app tests.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PlanetInteractionInput {
    pub cursor_ray: Option<PlanetCursorRay>,
    pub primary_just_pressed: bool,
}

/// Observable state for the currently hovered planet.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlanetInteractionState {
    pub hovered_planet: Option<PlanetId>,
    pub selected_planet: Option<PlanetId>,
}

/// The selected planet details shown in the destination card.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlanetCardState {
    pub planet_id: Option<PlanetId>,
}

#[derive(Component, Debug)]
pub struct PlanetCardName;

#[derive(Component, Debug)]
pub struct PlanetCardDetail;

#[derive(Component, Debug)]
pub struct PlanetCardVisual;

/// UI label that follows a planet while the cursor is over it.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlanetHoverLabel {
    pub planet_entity: Entity,
    pub planet_id: PlanetId,
}

/// Emissive mesh placed around a planet while it is selected.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlanetSelectionHighlight {
    pub planet_id: PlanetId,
}

pub struct PlanetInteractionPlugin;

impl Plugin for PlanetInteractionPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TransformPlugin>() {
            app.add_plugins(TransformPlugin);
        }

        app.init_resource::<PlanetInteractionInput>()
            .init_resource::<PlanetInteractionState>()
            .init_resource::<PlanetCardState>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(
                PostStartup,
                (
                    setup_planet_hover_labels,
                    setup_selection_highlights,
                    setup_planet_card,
                ),
            )
            .add_systems(
                Update,
                (
                    capture_window_pointer,
                    update_hovered_planet,
                    sync_selection_highlights,
                    update_planet_card,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                position_hover_labels.after(TransformSystems::Propagate),
            );
    }
}

fn setup_planet_card(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(42),
                top: px(42),
                width: px(286),
                padding: UiRect::all(px(18)),
                flex_direction: FlexDirection::Column,
                row_gap: px(9),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(CARD_BACKGROUND),
            ZIndex(20),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new("DESTINATION"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(CARD_QUIET),
            ));
            card.spawn((
                Node {
                    align_self: AlignSelf::Center,
                    width: px(96),
                    height: px(96),
                    margin: UiRect::vertical(px(4)),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(48)),
                    ..default()
                },
                BackgroundColor(PlanetId::Vesper.profile().card_color),
                BorderColor::all(Color::srgba(1.0, 0.9, 0.76, 0.62)),
                BoxShadow::new(
                    Color::srgba(0.35, 0.74, 1.0, 0.42),
                    px(0),
                    px(0),
                    px(3),
                    px(22),
                ),
                PlanetCardVisual,
            ))
            .with_children(|planet| {
                planet.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(19),
                        top: px(15),
                        width: px(22),
                        height: px(16),
                        border_radius: BorderRadius::all(px(11)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.48)),
                ));
            });
            card.spawn((
                Text::new("Select a planet"),
                TextFont {
                    font_size: FontSize::Px(25.0),
                    ..default()
                },
                TextColor(CARD_PRIMARY),
                PlanetCardName,
            ));
            card.spawn((
                Text::new("Hover over a planet for a closer look."),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(CARD_SECONDARY),
                PlanetCardDetail,
            ));
        });
}

fn setup_selection_highlights(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    planets: Query<(Entity, &PlanetMarker)>,
) {
    let highlight_mesh = meshes.add(Sphere::new(0.43).mesh().ico(3).unwrap());
    let highlight_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.72, 0.32, 0.32),
        emissive: LinearRgba::rgb(4.0, 1.4, 0.25),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    for (planet_entity, planet) in &planets {
        commands.entity(planet_entity).with_children(|children| {
            children.spawn((
                Mesh3d(highlight_mesh.clone()),
                MeshMaterial3d(highlight_material.clone()),
                Transform::IDENTITY,
                Visibility::Hidden,
                Pickable::IGNORE,
                PlanetSelectionHighlight {
                    planet_id: planet.id,
                },
            ));
        });
    }
}

fn setup_planet_hover_labels(mut commands: Commands, planets: Query<(Entity, &PlanetMarker)>) {
    for (planet_entity, planet) in &planets {
        let profile = planet.id.profile();
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                padding: UiRect::axes(px(10), px(6)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(HOVER_LABEL_BACKGROUND),
            Text::new(profile.name),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(HOVER_LABEL_COLOR),
            PlanetHoverLabel {
                planet_entity,
                planet_id: planet.id,
            },
            ZIndex(20),
        ));
    }
}

fn capture_window_pointer(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut input: ResMut<PlanetInteractionInput>,
) {
    let Ok(window) = windows.single() else {
        // Keeping manually submitted input available makes the same app behavior
        // usable in headless tests, where no window exists.
        return;
    };

    input.primary_just_pressed = buttons.just_pressed(MouseButton::Left);
    input.cursor_ray = window.cursor_position().and_then(|cursor_position| {
        cameras.iter().find_map(|(camera, camera_transform)| {
            camera
                .viewport_to_world(camera_transform, cursor_position)
                .ok()
                .map(|ray| PlanetCursorRay {
                    origin: ray.origin,
                    direction: *ray.direction,
                })
        })
    });
}

fn update_hovered_planet(
    mut input: ResMut<PlanetInteractionInput>,
    mut state: ResMut<PlanetInteractionState>,
    mut card: ResMut<PlanetCardState>,
    planets: Query<(&PlanetMarker, &GlobalTransform)>,
) {
    state.hovered_planet = input.cursor_ray.and_then(|ray| {
        let direction = ray.direction.try_normalize()?;
        planets
            .iter()
            .filter_map(|(planet, transform)| {
                ray_sphere_distance(ray.origin, direction, transform.translation(), 0.38)
                    .map(|distance| (distance, planet.id))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, planet_id)| planet_id)
    });
    if input.primary_just_pressed {
        if let Some(planet_id) = state.hovered_planet {
            state.selected_planet = Some(planet_id);
            card.planet_id = Some(planet_id);
        }
    }
    input.primary_just_pressed = false;
}

fn update_planet_card(
    card: Res<PlanetCardState>,
    mut names: Query<&mut Text, With<PlanetCardName>>,
    mut details: Query<&mut Text, (With<PlanetCardDetail>, Without<PlanetCardName>)>,
    mut visuals: Query<&mut BackgroundColor, With<PlanetCardVisual>>,
) {
    let profile = card.planet_id.map(PlanetId::profile);
    for mut text in &mut names {
        text.0 = profile
            .map_or("Select a planet", |profile| profile.name)
            .to_owned();
    }
    for mut text in &mut details {
        text.0 = profile
            .map_or("Hover over a planet for a closer look.", |profile| {
                profile.detail
            })
            .to_owned();
    }
    for mut background in &mut visuals {
        background.0 = profile.map_or(CARD_PRIMARY, |profile| profile.card_color);
    }
}

fn sync_selection_highlights(
    state: Res<PlanetInteractionState>,
    mut highlights: Query<(&PlanetSelectionHighlight, &mut Visibility)>,
) {
    for (highlight, mut visibility) in &mut highlights {
        *visibility = if state.selected_planet == Some(highlight.planet_id) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn ray_sphere_distance(origin: Vec3, direction: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let offset = origin - center;
    let projected_offset = offset.dot(direction);
    let discriminant =
        projected_offset * projected_offset - (offset.length_squared() - radius * radius);
    if discriminant < 0.0 {
        return None;
    }

    let near_distance = -projected_offset - discriminant.sqrt();
    let far_distance = -projected_offset + discriminant.sqrt();
    if near_distance >= 0.0 {
        Some(near_distance)
    } else if far_distance >= 0.0 {
        Some(far_distance)
    } else {
        None
    }
}

fn position_hover_labels(
    state: Res<PlanetInteractionState>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    planets: Query<&GlobalTransform, With<PlanetMarker>>,
    mut labels: Query<(&PlanetHoverLabel, &mut Node)>,
) {
    let camera = cameras.iter().next();

    for (label, mut node) in &mut labels {
        if state.hovered_planet != Some(label.planet_id) {
            node.display = Display::None;
            continue;
        }

        node.display = Display::Flex;
        let Some((camera, camera_transform)) = camera else {
            continue;
        };
        let Ok(planet_transform) = planets.get(label.planet_entity) else {
            continue;
        };

        if let Ok(screen_position) =
            camera.world_to_viewport(camera_transform, planet_transform.translation())
        {
            node.left = px(screen_position.x + 16.0);
            node.top = px(screen_position.y + 12.0);
        } else {
            // A stable fallback also gives headless app tests a visible label
            // state when no render target has computed a viewport.
            node.left = px(470);
            node.top = px(72);
        }
    }
}
