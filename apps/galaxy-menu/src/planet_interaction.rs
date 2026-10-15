//! Coordinates pointer capture, planet selection, and the presentation of interaction state.

use bevy::{
    picking::Pickable,
    prelude::*,
    transform::{TransformPlugin, TransformSystems},
    window::PrimaryWindow,
};

use crate::{PlanetId, PlanetMarker};

mod card;
mod hover_label;

pub use card::{PlanetCardDetail, PlanetCardName};
pub use hover_label::PlanetHoverLabel;

/// A world-space cursor ray. Its direction is normalized by the interaction system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetCursorRay {
    pub origin: Vec3,
    pub direction: Vec3,
}

/// Input seam shared by the window adapter and headless app tests.
///
/// When a primary window exists, the window adapter refreshes these fields each frame.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PlanetInteractionInput {
    pub cursor_ray: Option<PlanetCursorRay>,
    pub primary_just_pressed: bool,
}

/// Observable hover and selection state for app systems and tests.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlanetInteractionState {
    pub hovered_planet: Option<PlanetId>,
    pub selected_planet: Option<PlanetId>,
}

/// The selected destination that drives the destination card.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlanetCardState {
    pub planet_id: Option<PlanetId>,
}

/// Emissive mesh placed around a planet while it is selected.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlanetSelectionHighlight {
    pub planet_id: PlanetId,
}

/// Installs pointer capture, ray-based planet selection, and its UI feedback.
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
                    hover_label::setup_planet_hover_labels,
                    setup_selection_highlights,
                    card::setup_planet_card,
                ),
            )
            .add_systems(
                Update,
                // Capture first, then derive selection and synchronize its visible feedback.
                (
                    capture_window_pointer,
                    update_hovered_planet,
                    sync_selection_highlights,
                    card::update_planet_card,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                hover_label::position_hover_labels.after(TransformSystems::Propagate),
            );
    }
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
    // Normalize once so the ray/sphere distances compare correctly across planets.
    state.hovered_planet = input.cursor_ray.and_then(|ray| {
        let direction = ray.direction.try_normalize()?;
        planets
            .iter()
            .filter_map(|(planet, transform)| {
                ray_sphere_distance(ray.origin, direction, transform.translation(), 0.38)
                    .map(|distance| (distance, planet.id))
            })
            // The nearest non-negative hit wins, independent of ECS query order.
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

/// Returns the nearest non-negative distance along a normalized ray to a sphere.
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
