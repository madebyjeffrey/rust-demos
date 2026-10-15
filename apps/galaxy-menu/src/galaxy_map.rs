use bevy::{
    asset::RenderAssetUsages,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    mesh::PrimitiveTopology,
    picking::Pickable,
    post_process::bloom::Bloom,
    prelude::*,
    render::view::Tonemapping,
};

const GALAXY_SEED: u64 = 0xA57A_21C4_11D3_90B7;
const BACKGROUND_SEED: u64 = 0xBACC_5700_5EED_2026;
const GALAXY_STAR_COUNT: usize = 1_650;
const BACKGROUND_STAR_COUNT: usize = 650;
const GALAXY_RADIUS: f32 = 12.0;
const GALAXY_CENTER: Vec3 = Vec3::new(7.4, 0.0, 0.0);
const CAMERA_TARGET: Vec3 = Vec3::new(3.8, 0.0, 0.0);
const ORBIT_SENSITIVITY: f32 = 0.005;
const MIN_CAMERA_PITCH: f32 = -1.35;
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

/// A selectable destination represented by a simple glowing planet marker.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanetMarker {
    pub name: &'static str,
}

#[derive(Component, Debug)]
pub struct BackgroundStarfield;

#[derive(Component, Debug)]
struct GalaxyCamera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

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
        BACKGROUND_SEED,
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

    let galaxy_mesh = meshes.add(build_galaxy_starfield(GALAXY_SEED));
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
    let planet_definitions = [
        (
            "Vesper",
            Vec3::new(-5.8, 0.10, 3.2),
            Color::srgb(0.52, 0.81, 1.0),
            LinearRgba::rgb(1.8, 6.0, 11.0),
        ),
        (
            "Kestrel",
            Vec3::new(-2.1, -0.12, -4.2),
            Color::srgb(1.0, 0.59, 0.40),
            LinearRgba::rgb(11.0, 2.4, 0.7),
        ),
        (
            "Solace",
            Vec3::new(2.4, 0.14, 5.8),
            Color::srgb(0.79, 0.57, 1.0),
            LinearRgba::rgb(5.0, 1.8, 11.0),
        ),
        (
            "Meridian",
            Vec3::new(5.8, -0.08, 1.5),
            Color::srgb(1.0, 0.75, 0.43),
            LinearRgba::rgb(11.0, 5.0, 1.0),
        ),
        (
            "Halcyon",
            Vec3::new(3.8, 0.09, -7.2),
            Color::srgb(0.43, 0.91, 0.84),
            LinearRgba::rgb(0.8, 8.0, 6.4),
        ),
    ];

    commands.entity(root).with_children(|children| {
        for (name, position, base_color, emissive) in planet_definitions {
            let material = materials.add(StandardMaterial {
                base_color,
                emissive,
                ..default()
            });
            children.spawn((
                PlanetMarker { name },
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

fn rotate_galaxy_map(mut roots: Query<&mut Transform, With<GalaxyRoot>>, time: Res<Time>) {
    for mut root in &mut roots {
        root.rotate_y(IDLE_ROTATION_RADIANS_PER_SECOND * time.delta_secs());
    }
}

fn build_galaxy_starfield(seed: u64) -> Mesh {
    let mut rng = StableRandom::new(seed);
    let mut positions = Vec::with_capacity(GALAXY_STAR_COUNT * 24);
    let mut normals = Vec::with_capacity(GALAXY_STAR_COUNT * 24);
    let mut colors = Vec::with_capacity(GALAXY_STAR_COUNT * 24);

    for star in 0..GALAXY_STAR_COUNT {
        let radius = if star < GALAXY_STAR_COUNT / 3 {
            0.45 + GALAXY_RADIUS * 0.34 * rng.unit().powf(1.9)
        } else {
            1.3 + GALAXY_RADIUS * 0.89 * rng.unit().sqrt()
        };
        let arm = (star % 3) as f32;
        let angle = arm * std::f32::consts::TAU / 3.0
            + radius * 0.46
            + rng.signed() * (0.14 + radius * 0.009);
        let center = Vec3::new(
            radius * angle.cos(),
            rng.signed() * (0.09 + radius * 0.012),
            radius * angle.sin(),
        );
        let size = 0.025 + rng.unit() * 0.054;
        let color = galaxy_star_color(radius, &mut rng);
        append_octahedron(
            &mut positions,
            &mut normals,
            &mut colors,
            center,
            size,
            color,
        );
    }

    vertex_mesh(positions, normals, colors)
}

fn build_background_starfield(
    seed: u64,
    camera_right: Dir3,
    camera_up: Dir3,
    center: Vec3,
) -> Mesh {
    let mut rng = StableRandom::new(seed);
    let mut positions = Vec::with_capacity(BACKGROUND_STAR_COUNT * 24);
    let mut normals = Vec::with_capacity(BACKGROUND_STAR_COUNT * 24);
    let mut colors = Vec::with_capacity(BACKGROUND_STAR_COUNT * 24);

    for _ in 0..BACKGROUND_STAR_COUNT {
        let x = rng.signed() * 92.0;
        let y = rng.signed() * 54.0;
        let z = rng.signed() * 8.0;
        let camera_back = camera_right.as_vec3().cross(camera_up.as_vec3());
        let center = center + *camera_right * x + *camera_up * y + camera_back * z;
        let size = 0.025 + rng.unit() * 0.055;
        let brightness = 0.6 + rng.unit() * 1.5;
        let tint = match rng.next_u64() % 4 {
            0 => [0.58, 0.72, 1.0],
            1 => [0.78, 0.84, 1.0],
            2 => [1.0, 0.79, 0.63],
            _ => [1.0, 1.0, 1.0],
        };
        let color = [
            tint[0] * brightness,
            tint[1] * brightness,
            tint[2] * brightness,
            1.0,
        ];
        append_octahedron(
            &mut positions,
            &mut normals,
            &mut colors,
            center,
            size,
            color,
        );
    }

    vertex_mesh(positions, normals, colors)
}

fn galaxy_star_color(radius: f32, rng: &mut StableRandom) -> [f32; 4] {
    let tint = if radius < 2.4 {
        [1.0, 0.69, 0.43]
    } else {
        match rng.next_u64() % 4 {
            0 => [0.48, 0.68, 1.0],
            1 => [0.73, 0.49, 1.0],
            2 => [1.0, 0.49, 0.76],
            _ => [0.94, 0.85, 1.0],
        }
    };
    let brightness = if radius < 2.4 {
        1.35 + rng.unit() * 1.4
    } else {
        0.95 + rng.unit() * 1.4
    };
    [
        tint[0] * brightness,
        tint[1] * brightness,
        tint[2] * brightness,
        1.0,
    ]
}

fn vertex_mesh(positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, colors: Vec<[f32; 4]>) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}

fn append_octahedron(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    center: Vec3,
    radius: f32,
    color: [f32; 4],
) {
    let points = [
        center + Vec3::X * radius,
        center - Vec3::X * radius,
        center + Vec3::Y * radius,
        center - Vec3::Y * radius,
        center + Vec3::Z * radius,
        center - Vec3::Z * radius,
    ];
    let faces = [
        [2, 0, 4],
        [2, 4, 1],
        [2, 1, 5],
        [2, 5, 0],
        [3, 4, 0],
        [3, 1, 4],
        [3, 5, 1],
        [3, 0, 5],
    ];

    for [a_index, mut b_index, mut c_index] in faces {
        let a = points[a_index];
        let mut b = points[b_index];
        let mut c = points[c_index];
        let mut normal = (b - a).cross(c - a);
        if normal.dot((a + b + c) / 3.0 - center) < 0.0 {
            std::mem::swap(&mut b_index, &mut c_index);
            b = points[b_index];
            c = points[c_index];
            normal = (b - a).cross(c - a);
        }
        let normal = normal.normalize().to_array();
        for vertex in [a, b, c] {
            positions.push(vertex.to_array());
            normals.push(normal);
            colors.push(color);
        }
    }
}

struct StableRandom(u64);

impl StableRandom {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn unit(&mut self) -> f32 {
        ((self.next_u64() >> 40) as u32) as f32 / ((1_u32 << 24) as f32)
    }

    fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}
