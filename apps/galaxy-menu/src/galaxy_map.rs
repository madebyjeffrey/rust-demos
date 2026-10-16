//! Assembles the camera and stellar and gas geometry of the galaxy backdrop.

use std::collections::HashMap;

use bevy::{
    core_pipeline::prepass::DepthPrepass,
    input::{
        mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
        touch::TouchPhase,
    },
    picking::Pickable,
    post_process::bloom::Bloom,
    prelude::*,
    render::view::Tonemapping,
    transform::TransformPlugin,
};

use crate::{
    galaxy_companions::{GalaxyCompanion, spawn_galaxy_companions, update_companion_orbits},
    galaxy_gas::{
        GAS_HALF_HEIGHT, GalaxyDustUniform, GalaxyGas, GalaxyGasMaterial,
        build_galaxy_medium_volumes, register_gas_material,
    },
    menu::{BackdropTouchInput, MenuInputOwnership},
    starfield::{
        GALAXY_RADIUS, StarMaterial, build_background_starfield, build_galaxy_starfield,
        register_star_material,
    },
};

const GALAXY_CENTER: Vec3 = Vec3::new(7.4, 0.0, 0.0);
const CAMERA_TARGET: Vec3 = Vec3::new(3.8, 0.0, 0.0);
const ORBIT_SENSITIVITY: f32 = 0.005;
const MIN_CAMERA_DISTANCE: f32 = 18.0;
const MAX_CAMERA_DISTANCE: f32 = 72.0;
const ZOOM_SENSITIVITY: f32 = 0.12;
const IDLE_ROTATION_RADIANS_PER_SECOND: f32 = 0.035;
const FRAMING_VIEWPORT_FRACTION: f32 = 0.9;
// Include the outer stars' size and the full thickness of the gas.
const GALAXY_FRAMING_RADIUS: f32 = GALAXY_RADIUS + 0.1;
const GALAXY_FRAMING_HALF_HEIGHT: f32 = GAS_HALF_HEIGHT;

/// Stable transform anchor for the galaxy backdrop's rotating contents.
///
/// Later scene motion can rotate this entity without moving the camera or
/// background star field.
#[derive(Component, Debug)]
pub struct GalaxyRoot;

/// Marks the main galaxy's stellar geometry.
#[derive(Component, Debug)]
pub struct GalaxyDecorative;

/// Marks the stationary background stars surrounding the galaxy backdrop.
#[derive(Component, Debug)]
pub struct BackgroundStarfield;

#[derive(Component, Debug)]
struct GalaxyCamera {
    target: Vec3,
    orientation: Quat,
    distance: f32,
}

#[derive(Resource, Default)]
pub(super) struct TouchNavigation {
    positions: HashMap<(Entity, u64), Vec2>,
    ui_touch_active: bool,
}

impl TouchNavigation {
    pub(super) fn is_active(&self) -> bool {
        !self.positions.is_empty() || self.ui_touch_active
    }

    fn pinch_span(&self) -> Option<f32> {
        if self.positions.len() != 2 {
            return None;
        }
        let mut positions = self.positions.values();
        Some(positions.next()?.distance(*positions.next()?))
    }
}

type GalaxyCameraFilter = (Without<GalaxyRoot>, Without<GalaxyCompanion>);

/// Adds the backdrop and schedules free camera orbit, bounded zoom, and idle rotation.
pub struct GalaxyMapPlugin;

impl Plugin for GalaxyMapPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TransformPlugin>() {
            app.add_plugins(TransformPlugin);
        }
        register_gas_material(app);
        register_star_material(app);
        app.init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<TouchNavigation>()
            .init_resource::<Assets<Image>>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<AccumulatedMouseScroll>()
            .add_systems(Startup, setup_galaxy_map)
            .add_systems(
                Update,
                (
                    touch_galaxy_camera,
                    rotate_galaxy_map,
                    update_companion_orbits,
                    sync_galaxy_dust,
                    orbit_galaxy_camera,
                )
                    .chain(),
            );
    }
}

fn setup_galaxy_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut star_materials: ResMut<Assets<StarMaterial>>,
    mut gas_materials: ResMut<Assets<GalaxyGasMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let galaxy_transform =
        Transform::from_translation(GALAXY_CENTER).with_rotation(Quat::from_rotation_x(0.10));
    let dust_settings = GalaxyDustUniform::new(galaxy_transform.to_matrix().inverse());
    let (gas_volume, dust_volume) = build_galaxy_medium_volumes();
    let dust_texture = images.add(dust_volume);
    let camera_position = Vec3::new(
        CAMERA_TARGET.x,
        CAMERA_TARGET.y + 18.0,
        CAMERA_TARGET.z + 31.0,
    );
    let camera_offset = camera_position - CAMERA_TARGET;
    let camera_distance = camera_offset.length();
    let camera_transform =
        Transform::from_translation(camera_position).looking_at(CAMERA_TARGET, Vec3::Y);
    commands.spawn((
        Camera3d::default(),
        DepthPrepass,
        Bloom::NATURAL,
        Tonemapping::SomewhatBoringDisplayTransform,
        camera_transform,
        GalaxyCamera {
            target: CAMERA_TARGET,
            orientation: camera_transform.rotation,
            distance: camera_distance,
        },
    ));

    let background_mesh = meshes.add(build_background_starfield(CAMERA_TARGET));
    let background_material =
        star_materials.add(StarMaterial::new(true, dust_texture.clone(), dust_settings));
    commands.spawn((
        Mesh3d(background_mesh),
        MeshMaterial3d(background_material),
        BackgroundStarfield,
        Pickable::IGNORE,
    ));

    let galaxy_mesh = meshes.add(build_galaxy_starfield());
    let galaxy_material = star_materials.add(StarMaterial::new(
        false,
        dust_texture.clone(),
        dust_settings,
    ));
    spawn_galaxy_companions(
        &mut commands,
        &mut meshes,
        galaxy_material.clone(),
        GALAXY_CENTER,
    );
    let cloud_mesh = meshes.add(Cuboid::new(
        GALAXY_RADIUS * 2.0,
        GAS_HALF_HEIGHT * 2.0,
        GALAXY_RADIUS * 2.0,
    ));
    let cloud_material = gas_materials.add(GalaxyGasMaterial::new(
        images.add(gas_volume),
        dust_texture,
        dust_settings,
    ));

    // Retain the initial composition while fitting to the whole window. Menu overlays
    // do not reserve a separate scene viewport or change the camera framing.
    let root = commands
        .spawn((GalaxyRoot, Visibility::default(), galaxy_transform))
        .id();
    commands.entity(root).with_children(|children| {
        children.spawn((
            Mesh3d(galaxy_mesh),
            MeshMaterial3d(galaxy_material),
            GalaxyDecorative,
            Pickable::IGNORE,
        ));
        children.spawn((
            Mesh3d(cloud_mesh),
            MeshMaterial3d(cloud_material),
            GalaxyGas,
            Pickable::IGNORE,
        ));
    });
}

fn sync_galaxy_dust(
    roots: Query<Ref<Transform>, With<GalaxyRoot>>,
    mut star_materials: ResMut<Assets<StarMaterial>>,
    mut gas_materials: ResMut<Assets<GalaxyGasMaterial>>,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    if !root.is_changed() {
        return;
    }
    let world_to_galaxy = root.to_matrix().inverse();
    for (_, material) in star_materials.iter_mut() {
        material.dust_settings.world_to_galaxy = world_to_galaxy;
    }
    for (_, material) in gas_materials.iter_mut() {
        material.dust_settings.world_to_galaxy = world_to_galaxy;
    }
}

fn touch_galaxy_camera(
    mut cameras: Query<&mut GalaxyCamera>,
    menu_input: Option<Res<MenuInputOwnership>>,
    mut touch_navigation: ResMut<TouchNavigation>,
) {
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };
    if let Some(input) = &menu_input {
        touch_navigation.ui_touch_active = input.has_active_touches();
        for event in &input.backdrop_touches {
            let BackdropTouchInput::Touch(touch) = event else {
                touch_navigation.positions.clear();
                continue;
            };
            let key = (touch.window, touch.id);
            match touch.phase {
                TouchPhase::Started => {
                    touch_navigation.positions.insert(key, touch.position);
                }
                TouchPhase::Moved => {
                    let previous_span = touch_navigation.pinch_span();
                    let count = touch_navigation.positions.len();
                    if let Some(previous) = touch_navigation.positions.get_mut(&key) {
                        let delta = (touch.position - *previous) * -ORBIT_SENSITIVITY;
                        *previous = touch.position;
                        if count == 1 {
                            let rotation =
                                Quat::from_rotation_y(delta.x) * Quat::from_rotation_x(delta.y);
                            camera.orientation = (camera.orientation * rotation).normalize();
                        }
                        if let (Some(previous), Some(current)) =
                            (previous_span, touch_navigation.pinch_span())
                            && previous > f32::EPSILON
                            && current > f32::EPSILON
                        {
                            camera.distance *= previous / current;
                        }
                    }
                }
                TouchPhase::Ended | TouchPhase::Canceled => {
                    touch_navigation.positions.remove(&key);
                }
            }
        }
    }
}

fn orbit_galaxy_camera(
    mut cameras: Query<(&mut Transform, &mut GalaxyCamera, &Projection), GalaxyCameraFilter>,
    galaxies: Query<&Transform, With<GalaxyRoot>>,
    companions: Query<(&Transform, &GalaxyCompanion)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    menu_input: Option<Res<MenuInputOwnership>>,
) {
    let Ok((mut transform, mut camera, projection)) = cameras.single_mut() else {
        return;
    };

    // Rotate around the current view axes so dragging stays continuous through the poles.
    if mouse_buttons.pressed(MouseButton::Left)
        && !menu_input.as_ref().is_some_and(|input| input.blocks_mouse)
    {
        let delta = mouse_motion.delta * -ORBIT_SENSITIVITY;
        let rotation = Quat::from_rotation_y(delta.x) * Quat::from_rotation_x(delta.y);
        camera.orientation = (camera.orientation * rotation).normalize();
    }

    let scroll = match mouse_scroll.unit {
        MouseScrollUnit::Line => mouse_scroll.delta.y,
        MouseScrollUnit::Pixel => mouse_scroll.delta.y / 100.0,
    };
    if scroll != 0.0 && !menu_input.as_ref().is_some_and(|input| input.blocks_scroll) {
        camera.distance *= (-scroll * ZOOM_SENSITIVITY).exp();
    }

    let direction = camera.orientation * Vec3::Z;
    let orientation = Transform::from_rotation(camera.orientation);
    let minimum_distance = galaxies.single().map_or(MIN_CAMERA_DISTANCE, |galaxy| {
        minimum_camera_distance(
            camera.target,
            &orientation,
            projection,
            galaxy,
            companions.iter(),
        )
    });
    if camera.is_added() {
        // Start just outside the fitted scene, leaving room for the first zoom gesture.
        camera.distance = camera.distance.max(minimum_distance * 1.12);
    }
    // Recheck every frame: orbit, idle rotation, and viewport resizing can all change the fit.
    camera.distance = camera
        .distance
        .clamp(minimum_distance, MAX_CAMERA_DISTANCE.max(minimum_distance));
    transform.translation = camera.target + direction * camera.distance;
    transform.rotation = orientation.rotation;
}

fn minimum_camera_distance<'a>(
    target: Vec3,
    orientation: &Transform,
    projection: &Projection,
    galaxy: &Transform,
    companions: impl Iterator<Item = (&'a Transform, &'a GalaxyCompanion)>,
) -> f32 {
    let Projection::Perspective(projection) = projection else {
        return MIN_CAMERA_DISTANCE;
    };
    let vertical_tangent = (projection.fov * 0.5).tan() * FRAMING_VIEWPORT_FRACTION;
    let horizontal_tangent = vertical_tangent * projection.aspect_ratio;
    let back = -*orientation.forward();
    let horizontal = *orientation.right() / horizontal_tangent;
    let vertical = *orientation.up() / vertical_tangent;

    // For each frustum side, solve the distance needed to contain a tilted disk.
    // Its support in a direction is the radial extent plus its vertical thickness.
    let planes = [
        back + horizontal,
        back - horizontal,
        back + vertical,
        back - vertical,
    ];
    let galaxy_distance = planes
        .into_iter()
        .map(|normal| {
            let local_normal = (galaxy.rotation.inverse() * normal) * galaxy.scale;
            normal.dot(galaxy.translation - target)
                + GALAXY_FRAMING_RADIUS * Vec2::new(local_normal.x, local_normal.z).length()
                + GALAXY_FRAMING_HALF_HEIGHT * local_normal.y.abs()
        })
        .fold(MIN_CAMERA_DISTANCE, f32::max);

    companions
        .flat_map(|(transform, companion)| {
            planes.into_iter().map(move |normal| {
                normal.dot(transform.translation - target)
                    + companion.framing_radius * normal.length()
            })
        })
        .fold(galaxy_distance, f32::max)
}

fn rotate_galaxy_map(
    mut roots: Query<&mut Transform, With<GalaxyRoot>>,
    time: Res<Time>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    touch_navigation: Res<TouchNavigation>,
) {
    // Pause the idle motion while the user is manipulating the view.
    if mouse_buttons.pressed(MouseButton::Left)
        || mouse_motion.delta != Vec2::ZERO
        || mouse_scroll.delta != Vec2::ZERO
        || touch_navigation.is_active()
    {
        return;
    }

    for mut root in &mut roots {
        root.rotate_y(IDLE_ROTATION_RADIANS_PER_SECOND * time.delta_secs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotating_galaxy_keeps_stars_and_gas_sampling_the_same_dust() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(GalaxyMapPlugin);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);

        let local_star = Vec3::new(3.0, 0.1, -2.0);
        for rotation in [Quat::from_rotation_y(0.8), Quat::from_rotation_z(-0.6)] {
            let world = app.world_mut();
            world
                .query_filtered::<&mut Transform, With<GalaxyRoot>>()
                .single_mut(world)
                .unwrap()
                .rotation = rotation;
            app.update();
            let world = app.world_mut();
            let root = world
                .query_filtered::<&Transform, With<GalaxyRoot>>()
                .single(world)
                .unwrap();
            let world_star = root.to_matrix().transform_point3(local_star);
            for (_, material) in world.resource::<Assets<StarMaterial>>().iter() {
                let sampled = material
                    .dust_settings
                    .world_to_galaxy
                    .transform_point3(world_star);
                assert!(
                    sampled.distance(local_star) < 0.0001,
                    "stellar extinction must follow the rotating dust lane"
                );
            }
            for (_, material) in world.resource::<Assets<GalaxyGasMaterial>>().iter() {
                let sampled = material
                    .dust_settings
                    .world_to_galaxy
                    .transform_point3(world_star);
                assert!(
                    sampled.distance(local_star) < 0.0001,
                    "diffuse light must sample dust at the same galaxy position as the stars"
                );
            }
        }
    }
}
