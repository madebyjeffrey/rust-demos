//! Continuous, procedural starlight, gas, and absorbing dust in the rotating galaxy disk.

use bevy::{
    asset::{AssetPlugin, RenderAssetUsages, embedded_asset},
    image::ImageSampler,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
        TextureDimension, TextureFormat,
    },
    shader::{ShaderRef, load_shader_library},
};

use crate::starfield::{GALAXY_RADIUS, GALAXY_THICKNESS_SCALE, galaxy_edge_fade, spiral_angle};

const TEXTURE_SIZE: u32 = 256;
// Retain the dust lane's vertical detail while giving faint bulge light more room to fade.
const TEXTURE_HEIGHT: u32 = 128;
pub(super) const GAS_HALF_HEIGHT: f32 = 2.25 * GALAXY_THICKNESS_SCALE;

/// The decorative gas volume shares the galaxy root, so its dust lanes rotate with the stars.
#[derive(Component, Debug)]
pub struct GalaxyGas;

/// Both star points and diffuse light trace the same dust field in galaxy coordinates.
#[derive(Clone, Copy, Debug, ShaderType)]
pub(super) struct GalaxyDustUniform {
    pub world_to_galaxy: Mat4,
    bounds_and_density: Vec4,
    absorption: Vec4,
}

impl GalaxyDustUniform {
    pub fn new(world_to_galaxy: Mat4) -> Self {
        Self {
            world_to_galaxy,
            bounds_and_density: Vec4::new(GALAXY_RADIUS, GAS_HALF_HEIGHT, GALAXY_RADIUS, 3.0),
            // Blue light is absorbed more strongly, leaving warmer light around dense dust.
            absorption: Vec4::new(4.0, 5.2, 7.0, 0.65 * GALAXY_THICKNESS_SCALE),
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(super) struct GalaxyGasMaterial {
    #[uniform(0)]
    pub dust_settings: GalaxyDustUniform,
    #[texture(1, dimension = "3d")]
    #[sampler(2)]
    dust: Handle<Image>,
    #[texture(3, dimension = "3d")]
    #[sampler(4)]
    volume: Handle<Image>,
}

impl GalaxyGasMaterial {
    pub fn new(
        volume: Handle<Image>,
        dust: Handle<Image>,
        dust_settings: GalaxyDustUniform,
    ) -> Self {
        Self {
            dust_settings,
            dust,
            volume,
        }
    }
}

pub(super) fn register_gas_material(app: &mut App) {
    if app.is_plugin_added::<AssetPlugin>() {
        load_shader_library!(app, "galaxy_medium.wesl");
        embedded_asset!(app, "galaxy_gas.wesl");
        app.add_plugins(MaterialPlugin::<GalaxyGasMaterial>::default());
    } else {
        app.init_resource::<Assets<GalaxyGasMaterial>>();
    }
}

impl Material for GalaxyGasMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://galaxy_menu/galaxy_gas.wesl".into()
    }

    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // Stars account for their own foreground dust, so only add diffuse emission here.
        AlphaMode::Add
    }

    fn depth_bias(&self) -> f32 {
        // Composite the gas after the transparent star mesh at the same scene origin.
        -0.1
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
        descriptor.vertex.buffers = vec![
            layout
                .0
                .get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0)])?,
        ];
        Ok(())
    }
}

#[derive(Default)]
struct CloudColumn {
    disk: f32,
    bulge: f32,
    halo: f32,
    dust: f32,
    dust_offset: f32,
    dust_thickness: f32,
    edge: f32,
}

/// Store a smooth density field in a trilinearly filtered texture, without geometric slices.
pub(super) fn build_galaxy_medium_volumes() -> (Image, Image) {
    let mut columns = Vec::with_capacity((TEXTURE_SIZE * TEXTURE_SIZE) as usize);
    for z in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let point = (Vec2::new(x as f32 + 0.5, z as f32 + 0.5) / TEXTURE_SIZE as f32 * 2.0
                - Vec2::ONE)
                * GALAXY_RADIUS;
            columns.push(cloud_column(point));
        }
    }

    let mut pixels = vec![0; (TEXTURE_SIZE * TEXTURE_HEIGHT * TEXTURE_SIZE * 4) as usize];
    let mut dust_pixels = vec![0; (TEXTURE_SIZE * TEXTURE_HEIGHT * TEXTURE_SIZE) as usize];
    for y in 0..TEXTURE_HEIGHT {
        let height = (2.0 * (y as f32 + 0.5) / TEXTURE_HEIGHT as f32 - 1.0) * GAS_HALF_HEIGHT
            / GALAXY_THICKNESS_SCALE;
        let disk_height = (-0.5 * (height / 0.16).powi(2)).exp();
        let bulge_height = (-height * height / 0.48).exp();
        let halo_height = (-height * height / 1.2).exp();
        let height_edge = 1.0 - smoothstep(1.1, 2.25, height.abs());
        for (column_index, column) in columns.iter().enumerate() {
            if column.edge == 0.0 {
                continue;
            }
            let disk = column.disk * disk_height;
            let bulge = column.bulge * bulge_height + column.halo * halo_height;
            let dust_height =
                (-0.5 * ((height - column.dust_offset) / column.dust_thickness).powi(2)).exp();
            let dust = column.dust * dust_height * column.edge * height_edge;
            let light = bulge + disk;
            let warm = Vec3::new(1.0, 0.86, 0.67);
            let cool = Vec3::new(0.59, 0.72, 0.96);
            let tint = (warm * bulge + cool * disk) / light.max(0.0001);
            let brightness = (0.12 + light * 0.44).min(1.0);
            let color = tint * brightness;
            let alpha = 1.0 - (-light * 0.48 * column.edge * height_edge).exp();
            let x = column_index % TEXTURE_SIZE as usize;
            let z = column_index / TEXTURE_SIZE as usize;
            let index =
                ((z * TEXTURE_HEIGHT as usize + y as usize) * TEXTURE_SIZE as usize + x) * 4;
            dust_pixels[index / 4] = (dust.clamp(0.0, 1.0) * 255.0) as u8;
            pixels[index..index + 4].copy_from_slice(&[
                (color.x.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8,
                (color.y.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8,
                (color.z.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8,
                // Companding preserves very faint opacity in this 8-bit texture.
                // Decode after filtering in the shader, avoiding a visible zero-opacity contour.
                (alpha.sqrt() * 255.0) as u8,
            ]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_HEIGHT,
            depth_or_array_layers: TEXTURE_SIZE,
        },
        TextureDimension::D3,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    let mut dust_image = Image::new(
        image.texture_descriptor.size,
        TextureDimension::D3,
        dust_pixels,
        TextureFormat::R8Unorm,
        RenderAssetUsages::default(),
    );
    dust_image.sampler = ImageSampler::linear();
    (image, dust_image)
}

fn cloud_column(point: Vec2) -> CloudColumn {
    let radius = point.length();
    if radius >= GALAXY_RADIUS {
        return CloudColumn::default();
    }
    let clouds = fractal_noise(point * 0.7);
    let phase = point.y.atan2(point.x) - spiral_angle(radius) + (clouds - 0.5) * 0.7;
    let arms = (-phase.sin().powi(2) / 0.095).exp();
    let spurs = (-(phase + 1.1).sin().powi(2) / 0.14).exp() * 0.32;
    let dust_clouds = fractal_noise(point * 1.8 + Vec2::splat(31.0));
    CloudColumn {
        disk: (0.22 + (arms + spurs) * (0.12 + clouds.powi(2) * 5.0)) * (-radius / 5.8).exp(),
        bulge: 3.2 * (-radius * radius / 3.0).exp(),
        halo: 0.13 * (-radius * radius / 22.0).exp(),
        // Dark filaments follow, and absorb light along, the bright arm ridge.
        dust: (0.16
            + arms * 0.22
            + (-(phase + 0.2).sin().powi(2) / 0.014).exp()
            + arms * smoothstep(0.47, 0.72, dust_clouds) * 0.6)
            * smoothstep(0.8, 2.5, radius)
            * (0.5 + dust_clouds)
            * (-radius / 9.0).exp(),
        dust_offset: (fractal_noise(point * 0.35 + Vec2::splat(53.0)) - 0.5) * 0.25,
        dust_thickness: 0.065 + dust_clouds * 0.1,
        edge: galaxy_edge_fade(radius),
    }
}

fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
    let t = ((value - start) / (end - start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn fractal_noise(point: Vec2) -> f32 {
    value_noise(point) * 0.52
        + value_noise(point * 2.03 + Vec2::new(7.1, 2.8)) * 0.27
        + value_noise(point * 4.17 + Vec2::new(1.7, 9.2)) * 0.14
        + value_noise(point * 8.31) * 0.07
}

fn value_noise(point: Vec2) -> f32 {
    let cell = point.floor();
    let fraction = point - cell;
    let blend = fraction * fraction * (Vec2::splat(3.0) - fraction * 2.0);
    let hash = |x: f32, y: f32| {
        let mut value = (x as i32 as u32).wrapping_mul(0x045D_9F3B)
            ^ (y as i32 as u32).wrapping_mul(0x27D4_EB2D)
            ^ 0xA57A_21C4;
        value = (value ^ (value >> 16)).wrapping_mul(0x7FEB_352D);
        value = (value ^ (value >> 15)).wrapping_mul(0x846C_A68B);
        (value ^ (value >> 16)) as f32 / u32::MAX as f32
    };
    let top = hash(cell.x, cell.y) * (1.0 - blend.x) + hash(cell.x + 1.0, cell.y) * blend.x;
    let bottom =
        hash(cell.x, cell.y + 1.0) * (1.0 - blend.x) + hash(cell.x + 1.0, cell.y + 1.0) * blend.x;
    top * (1.0 - blend.y) + bottom * blend.y
}
