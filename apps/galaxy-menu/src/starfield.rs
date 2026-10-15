//! Deterministic mesh generation for the galaxy and its stationary backdrop.

use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology, prelude::*};

const GALAXY_SEED: u64 = 0xA57A_21C4_11D3_90B7;
const BACKGROUND_SEED: u64 = 0xBACC_5700_5EED_2026;
const GALAXY_STAR_COUNT: usize = 1_650;
const BACKGROUND_STAR_COUNT: usize = 650;
const GALAXY_RADIUS: f32 = 12.0;
const OCTAHEDRON_VERTEX_COUNT: usize = 24;

/// Builds the fixed-seed spiral geometry in the galaxy root's local space.
pub(super) fn build_galaxy_starfield() -> Mesh {
    let mut rng = StableRandom::new(GALAXY_SEED);
    let mut mesh = StarMeshBuilder::with_capacity(GALAXY_STAR_COUNT);

    for star in 0..GALAXY_STAR_COUNT {
        // Concentrate the inner third around the core; distribute the rest along three arms.
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
        mesh.push_octahedron(center, size, color);
    }

    mesh.build()
}

/// Builds a fixed-seed field in the initial camera plane, independent of galaxy rotation.
pub(super) fn build_background_starfield(
    camera_right: Dir3,
    camera_up: Dir3,
    center: Vec3,
) -> Mesh {
    let mut rng = StableRandom::new(BACKGROUND_SEED);
    let mut mesh = StarMeshBuilder::with_capacity(BACKGROUND_STAR_COUNT);
    let camera_back = camera_right.as_vec3().cross(camera_up.as_vec3());

    for _ in 0..BACKGROUND_STAR_COUNT {
        let x = rng.signed() * 92.0;
        let y = rng.signed() * 54.0;
        let z = rng.signed() * 8.0;
        let position = center + *camera_right * x + *camera_up * y + camera_back * z;
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
        mesh.push_octahedron(position, size, color);
    }

    mesh.build()
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

/// Collects the per-vertex buffers shared by both procedural starfields.
struct StarMeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
}

impl StarMeshBuilder {
    fn with_capacity(star_count: usize) -> Self {
        let vertex_capacity = star_count * OCTAHEDRON_VERTEX_COUNT;
        Self {
            positions: Vec::with_capacity(vertex_capacity),
            normals: Vec::with_capacity(vertex_capacity),
            colors: Vec::with_capacity(vertex_capacity),
        }
    }

    fn push_octahedron(&mut self, center: Vec3, radius: f32, color: [f32; 4]) {
        append_octahedron(
            &mut self.positions,
            &mut self.normals,
            &mut self.colors,
            center,
            radius,
            color,
        );
    }

    fn build(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
    }
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

/// Small SplitMix64 generator keeps generated geometry stable without a dependency.
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
