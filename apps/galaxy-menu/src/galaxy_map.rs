//! Assembles the camera, galaxy geometry, and destination markers into the scene.

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    picking::Pickable,
    post_process::bloom::Bloom,
    prelude::*,
    render::view::Tonemapping,
};

use crate::{
    planet::{PlanetId, PlanetMarker},
    starfield::{build_background_starfield, build_galaxy_starfield},
};

const GALAXY_CENTER: Vec3 = Vec3::new(7.4, 0.0, 0.0);
const CAMERA_TARGET: Vec3 = Vec3::new(3.8, 0.0, 0.0);
const ORBIT_SENSITIVITY: f32 = 0.005;
const MIN_CAMERA_PITCH: f32 = 0.25;
const MAX_CAMERA_PITCH: f32 = 1.35;
const MIN_CAMERA_DISTANCE: f32 = 18.0;
const MAX_CAMERA_DISTANCE: f32 = 72.0;
const ZOOM_SENSITIVITY: f32 = 0.12;
const IDLE_ROTATION_RADIANS_PER_SECOND: f32 = 0.035;

/// Stable transform anchor for the galaxy map's contents.
///
/// Later scene motion can rotate this entity without moving the camera or
/// background star field.
#[derive(Component, Debug)]
pub struct GalaxyRoot;

/// Marks decorative geometry that should never intercept planet picking.
#[derive(Component, Debug)]
pub struct GalaxyDecorative;

/// Marks the stationary backdrop so it can be excluded from picking and found in tests.
#[derive(Component, Debug)]
pub struct BackgroundStarfield;

#[derive(Component, Debug)]
struct GalaxyCamera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

/// Adds the scene and schedules bounded camera movement plus idle map rotation.
pub struct GalaxyMapPlugin;

impl Plugin for GalaxyMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<AccumulatedMouseScroll>()
            .add_systems(Startup, setup_galaxy_map)
            .add_systems(Update, (orbit_galaxy_camera, rotate_galaxy_map));
    }
}

fn setup_galaxy_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let camera_position = Vec3::new(
        CAMERA_TARGET.x,
        CAMERA_TARGET.y + 22.0,
        CAMERA_TARGET.z + 37.0,
    );
    let camera_offset = camera_position - CAMERA_TARGET;
    let camera_distance = camera_offset.length();
    let camera_yaw = camera_offset.x.atan2(camera_offset.z);
    let camera_pitch = camera_offset
        .y
        .atan2(Vec2::new(camera_offset.x, camera_offset.z).length());
    let camera_transform =
        Transform::from_translation(camera_position).looking_at(CAMERA_TARGET, Vec3::Y);
    let camera_forward = camera_transform.forward();
    let camera_right = camera_transform.right();
    let camera_up = camera_transform.up();

    commands.spawn((
        Camera3d::default(),
        Bloom::NATURAL,
        Tonemapping::SomewhatBoringDisplayTransform,
        camera_transform,
        GalaxyCamera {
            target: CAMERA_TARGET,
            yaw: camera_yaw,
            pitch: camera_pitch,
            distance: camera_distance,
        },
    ));

    let background_center = CAMERA_TARGET + camera_forward * 70.0;
    let background_mesh = meshes.add(build_background_starfield(
        camera_right,
        camera_up,
        background_center,
    ));
    let background_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(background_mesh),
        MeshMaterial3d(background_material),
        BackgroundStarfield,
        Pickable::IGNORE,
    ));

    let galaxy_mesh = meshes.add(build_galaxy_starfield());
    let galaxy_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });
    let core_mesh = meshes.add(Sphere::new(1.15).mesh().ico(4).unwrap());
    let core_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.61, 0.31),
        emissive: LinearRgba::rgb(5.0, 1.8, 0.45),
        ..default()
    });

    // Place the galaxy right of the camera target to leave a clear area for the menu on the left.
    // Parenting its geometry and planets together keeps idle rotation local to the map.
    let root = commands
        .spawn((
            GalaxyRoot,
            Visibility::default(),
            Transform::from_translation(GALAXY_CENTER).with_rotation(Quat::from_rotation_x(0.10)),
        ))
        .id();
    commands.entity(root).with_children(|children| {
        children.spawn((
            Mesh3d(galaxy_mesh),
            MeshMaterial3d(galaxy_material),
            GalaxyDecorative,
            Pickable::IGNORE,
        ));
        children.spawn((
            Mesh3d(core_mesh),
            MeshMaterial3d(core_material),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            Pickable::IGNORE,
        ));
    });

    let planet_mesh = meshes.add(Sphere::new(0.32).mesh().ico(3).unwrap());
    // Positions and marker lighting are scene data; names and card copy live with PlanetId.
    let planet_definitions = [
        (
            PlanetId::Vesper,
            Vec3::new(-5.8, 0.10, 3.2),
            Color::srgb(0.52, 0.81, 1.0),
            LinearRgba::rgb(1.8, 6.0, 11.0),
        ),
        (
            PlanetId::Kestrel,
            Vec3::new(-2.1, -0.12, -4.2),
            Color::srgb(1.0, 0.59, 0.40),
            LinearRgba::rgb(11.0, 2.4, 0.7),
        ),
        (
            PlanetId::Solace,
            Vec3::new(2.4, 0.14, 5.8),
            Color::srgb(0.79, 0.57, 1.0),
            LinearRgba::rgb(5.0, 1.8, 11.0),
        ),
        (
            PlanetId::Meridian,
            Vec3::new(5.8, -0.08, 1.5),
            Color::srgb(1.0, 0.75, 0.43),
            LinearRgba::rgb(11.0, 5.0, 1.0),
        ),
        (
            PlanetId::Halcyon,
            Vec3::new(3.8, 0.09, -7.2),
            Color::srgb(0.43, 0.91, 0.84),
            LinearRgba::rgb(0.8, 8.0, 6.4),
        ),
    ];

    commands.entity(root).with_children(|children| {
        for (id, position, base_color, emissive) in planet_definitions {
            let material = materials.add(StandardMaterial {
                base_color,
                emissive,
                ..default()
            });
            children.spawn((
                PlanetMarker { id },
                Mesh3d(planet_mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(position),
            ));
        }
    });
}

fn orbit_galaxy_camera(
    mut cameras: Query<(&mut Transform, &mut GalaxyCamera)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    let Ok((mut transform, mut camera)) = cameras.single_mut() else {
        return;
    };

    // Drag changes yaw and pitch; scroll changes distance. Both remain bounded.
    if mouse_buttons.pressed(MouseButton::Left) {
        camera.yaw -= mouse_motion.delta.x * ORBIT_SENSITIVITY;
        camera.pitch = (camera.pitch + mouse_motion.delta.y * ORBIT_SENSITIVITY)
            .clamp(MIN_CAMERA_PITCH, MAX_CAMERA_PITCH);
    }

    let scroll = match mouse_scroll.unit {
        MouseScrollUnit::Line => mouse_scroll.delta.y,
        MouseScrollUnit::Pixel => mouse_scroll.delta.y / 100.0,
    };
    if scroll != 0.0 {
        camera.distance = (camera.distance * (-scroll * ZOOM_SENSITIVITY).exp())
            .clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);
    }

    let horizontal_distance = camera.distance * camera.pitch.cos();
    let offset = Vec3::new(
        horizontal_distance * camera.yaw.sin(),
        camera.distance * camera.pitch.sin(),
        horizontal_distance * camera.yaw.cos(),
    );
    transform.translation = camera.target + offset;
    transform.look_at(camera.target, Vec3::Y);
}

fn rotate_galaxy_map(
    mut roots: Query<&mut Transform, With<GalaxyRoot>>,
    time: Res<Time>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    // Pause the idle motion while the user is manipulating the view.
    if mouse_buttons.pressed(MouseButton::Left)
        || mouse_motion.delta != Vec2::ZERO
        || mouse_scroll.delta != Vec2::ZERO
    {
        return;
    }

    for mut root in &mut roots {
        root.rotate_y(IDLE_ROTATION_RADIANS_PER_SECOND * time.delta_secs());
    }
}
