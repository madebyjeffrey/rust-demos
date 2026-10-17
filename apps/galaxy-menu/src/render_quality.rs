//! Scene resolution can be reduced independently of native menu and input coordinates.
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    image::ImageSampler,
    picking::Pickable,
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    ui::IsDefaultUiCamera,
    window::PrimaryWindow,
};

/// Rendering policy for the shared app. Insert before adding `GalaxyMenuPlugin`
/// to override the platform default, or update it to compare release captures.
/// This affects the galaxy backdrop; window resolution and UI scale stay native.
#[derive(Resource, Clone, Copy, Debug)]
pub struct GalaxyRenderSettings {
    /// Fraction of native pixels along each scene dimension, from 0.1 through 1.
    pub scene_scale: f32,
}

impl Default for GalaxyRenderSettings {
    fn default() -> Self {
        Self {
            // Physical release captures missed the 60 Hz budget at native resolution.
            scene_scale: if cfg!(target_os = "ios") { 0.6 } else { 1.0 },
        }
    }
}

#[derive(Resource)]
struct SceneTarget {
    image: Handle<Image>,
    sprite: Entity,
    camera: Entity,
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<GalaxyRenderSettings>()
        .add_systems(PreUpdate, update_scene_target);
}

fn update_scene_target(
    mut commands: Commands,
    settings: Res<GalaxyRenderSettings>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scene: Query<(&mut Camera, &mut RenderTarget), With<Camera3d>>,
    target: Option<Res<SceneTarget>>,
    mut images: ResMut<Assets<Image>>,
    mut sprites: Query<&mut Sprite>,
) {
    let scale = settings.scene_scale;
    assert!(
        scale.is_finite() && (0.1..=1.0).contains(&scale),
        "scene_scale must be between 0.1 and 1"
    );
    let (Ok(window), Ok((mut camera, mut render_target))) = (windows.single(), scene.single_mut())
    else {
        return;
    };
    if scale == 1.0 {
        if let Some(target) = target {
            commands.entity(target.sprite).despawn();
            commands.entity(target.camera).despawn();
            *render_target = RenderTarget::default();
            camera.order = 0;
            commands.remove_resource::<SceneTarget>();
        }
        return;
    }
    let size = (window.physical_size().as_vec2() * scale)
        .round()
        .as_uvec2()
        .max(UVec2::ONE);
    if let Some(target) = target {
        // Do not mark the image modified or reallocate its GPU texture each frame.
        if images
            .get(&target.image)
            .is_some_and(|image| image.size() != size)
        {
            images.get_mut(&target.image).unwrap().resize(Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            });
        }
        if let Ok(mut sprite) = sprites.get_mut(target.sprite)
            && sprite.custom_size != Some(window.size())
        {
            sprite.custom_size = Some(window.size());
        }
        return;
    }

    let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
    image.sampler = ImageSampler::linear();
    let image = images.add(image);
    camera.order = -1;
    *render_target = RenderTarget::Image(image.clone().into());
    // The scene remains the only 3D camera. A separate native camera draws the
    // completed backdrop and menu, keeping text and touch targets at native scale.
    let layers = RenderLayers::layer(1);
    let composite_camera = commands
        .spawn((Camera2d, Msaa::Off, IsDefaultUiCamera, layers.clone()))
        .id();
    let sprite = commands
        .spawn((
            Sprite {
                image: image.clone(),
                custom_size: Some(window.size()),
                ..default()
            },
            layers,
            Pickable::IGNORE,
        ))
        .id();
    commands.insert_resource(SceneTarget {
        image,
        sprite,
        camera: composite_camera,
    });
}
