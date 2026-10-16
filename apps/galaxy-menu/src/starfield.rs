//! Deterministic mesh generation for the galaxy and its stationary backdrop.

use bevy::{
    asset::{AssetPlugin, RenderAssetUsages, embedded_asset},
    mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError, VertexFormat,
    },
    shader::ShaderRef,
};

use crate::galaxy_gas::GalaxyDustUniform;

const GALAXY_SEED: u64 = 0xA57A_21C4_11D3_90B7;
const BACKGROUND_SEED: u64 = 0xBACC_5700_5EED_2026;
const GALAXY_STAR_COUNT: usize = 64_000;
const BACKGROUND_STAR_COUNT: usize = 4_000;
pub(super) const GALAXY_RADIUS: f32 = 12.0;
// Keep the stars, gas volume, and camera framing in step as the disk gains depth.
pub(super) const GALAXY_THICKNESS_SCALE: f32 = 1.4;
const STAR_VERTEX_COUNT: usize = 4;
const ATTRIBUTE_STAR: MeshVertexAttribute =
    MeshVertexAttribute::new("StarCornerAndRadius", 0xA57A_2026, VertexFormat::Float32x3);

/// Soft, additive points keep subpixel stars visible without hard triangle coverage changes.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(super) struct StarMaterial {
    background: bool,
    #[uniform(0)]
    pub dust_settings: GalaxyDustUniform,
    #[texture(1, dimension = "3d")]
    #[sampler(2)]
    dust: Handle<Image>,
}

impl StarMaterial {
    pub fn new(background: bool, dust: Handle<Image>, dust_settings: GalaxyDustUniform) -> Self {
        Self {
            background,
            dust_settings,
            dust,
        }
    }
}

pub(super) fn register_star_material(app: &mut App) {
    // Headless scene tests only need the asset storage, without a renderer or asset server.
    if app.is_plugin_added::<AssetPlugin>() {
        embedded_asset!(app, "starfield.wesl");
        app.add_plugins(MaterialPlugin::<StarMaterial>::default());
    } else {
        app.init_resource::<Assets<StarMaterial>>();
    }
}

impl Material for StarMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://galaxy_menu/starfield.wesl".into()
    }

    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn depth_bias(&self) -> f32 {
        // Draw the enclosing shell before the galaxy's transparent gas and stars.
        if self.background { 1000.0 } else { 0.0 }
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(1),
            ATTRIBUTE_STAR.at_shader_location(2),
        ])?];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Builds the fixed-seed spiral geometry in the galaxy root's local space.
pub(super) fn build_galaxy_starfield() -> Mesh {
    let mut rng = StableRandom::new(GALAXY_SEED);
    let mut mesh = StarMeshBuilder::with_capacity(GALAXY_STAR_COUNT);

    for star in 0..GALAXY_STAR_COUNT {
        let mut center = if star % 5 == 0 {
            // A thick population of older stars gives the center a luminous bulge.
            Vec3::new(
                rng.normal() * 1.15,
                rng.normal() * 0.35,
                rng.normal() * 1.15,
            )
        } else {
            // An exponential disk remains populated between its broad, irregular arms.
            let radius = loop {
                let radius = -(rng.unit().max(0.0001) * rng.unit().max(0.0001)).ln() * 2.15;
                if radius < GALAXY_RADIUS - 0.2 {
                    break radius;
                }
            };
            let angle = if star % 3 != 0 {
                rng.unit() * std::f32::consts::TAU
            } else {
                let arm = (star % 2) as f32 * std::f32::consts::PI;
                arm + spiral_angle(radius) + rng.normal() * (0.23 + radius * 0.012)
            };
            Vec3::new(
                radius * angle.cos(),
                rng.normal() * (0.045 + radius * 0.006),
                radius * angle.sin(),
            )
        };
        center.y *= GALAXY_THICKNESS_SCALE;
        let bright = star % 173 == 0;
        let size = if bright {
            0.021 + rng.unit() * 0.014
        } else {
            0.005 + rng.unit().powi(3) * 0.014
        };
        let radius = center.xz().length();
        let mut color = galaxy_star_color(radius, bright, &mut rng);
        for channel in &mut color[..3] {
            *channel *= galaxy_edge_fade(radius);
        }
        mesh.push_star(center, size, color);
    }

    mesh.build()
}

/// Builds a stationary, fixed-seed star shell surrounding the camera's orbit.
pub(super) fn build_background_starfield(center: Vec3) -> Mesh {
    let mut rng = StableRandom::new(BACKGROUND_SEED);
    let mut mesh = StarMeshBuilder::with_capacity(BACKGROUND_STAR_COUNT);
    for _ in 0..BACKGROUND_STAR_COUNT {
        // Uniform directions give every orbit angle a similar density of stars.
        let y = rng.signed();
        let angle = rng.unit() * std::f32::consts::TAU;
        let horizontal = (1.0 - y * y).sqrt();
        let direction = Vec3::new(horizontal * angle.cos(), y, horizontal * angle.sin());
        let position = center + direction * (190.0 + rng.unit() * 20.0);
        let size = 0.07 + rng.unit() * 0.12;
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
        mesh.push_star(position, size, color);
    }

    mesh.build()
}

/// Shared winding keeps the young stars and gas on the same logarithmic arms.
pub(super) fn spiral_angle(radius: f32) -> f32 {
    3.3 * (1.0 + radius / 0.9).ln()
}

/// Dim the outskirts over a broad transition before the finite star distribution ends.
pub(super) fn galaxy_edge_fade(radius: f32) -> f32 {
    let t = ((radius - 7.8) / (GALAXY_RADIUS - 7.8)).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

fn galaxy_star_color(radius: f32, bright: bool, rng: &mut StableRandom) -> [f32; 4] {
    let tint = if radius < 2.4 {
        [1.0, 0.86, 0.66]
    } else {
        match rng.next_u64() % 4 {
            0 => [0.55, 0.72, 1.0],
            1 => [0.77, 0.85, 1.0],
            2 => [1.0, 0.88, 0.72],
            _ => [0.95, 0.97, 1.0],
        }
    };
    let brightness = if bright {
        3.0 + rng.unit() * 3.0
    } else {
        0.45 + rng.unit().powi(2) * 1.4
    };
    [
        tint[0] * brightness,
        tint[1] * brightness,
        tint[2] * brightness,
        1.0,
    ]
}

/// Collects the per-vertex buffers shared by both procedural starfields.
pub(super) struct StarMeshBuilder {
    positions: Vec<[f32; 3]>,
    corners_and_radii: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl StarMeshBuilder {
    pub fn with_capacity(star_count: usize) -> Self {
        let vertex_capacity = star_count * STAR_VERTEX_COUNT;
        Self {
            positions: Vec::with_capacity(vertex_capacity),
            corners_and_radii: Vec::with_capacity(vertex_capacity),
            colors: Vec::with_capacity(vertex_capacity),
            indices: Vec::with_capacity(star_count * 6),
        }
    }

    pub fn push_star(&mut self, center: Vec3, radius: f32, color: [f32; 4]) {
        // The vertex shader expands each center into a camera-facing, filtered point.
        let base = self.positions.len() as u32;
        for [x, y] in [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]] {
            self.positions.push(center.to_array());
            self.corners_and_radii.push([x, y, radius]);
            self.colors.push(color);
        }
        self.indices
            .extend([0, 1, 2, 0, 2, 3].map(|index| base + index));
    }

    pub fn build(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(ATTRIBUTE_STAR, self.corners_and_radii)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Small SplitMix64 generator keeps generated geometry stable without a dependency.
pub(super) struct StableRandom(u64);

impl StableRandom {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    pub fn unit(&mut self) -> f32 {
        ((self.next_u64() >> 40) as u32) as f32 / ((1_u32 << 24) as f32)
    }

    fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    pub fn normal(&mut self) -> f32 {
        (-2.0 * self.unit().max(0.0001).ln()).sqrt() * (std::f32::consts::TAU * self.unit()).cos()
    }
}
