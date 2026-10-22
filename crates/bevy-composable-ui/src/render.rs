use crate::{
    FrameError, SharedFrame,
    geometry::{MAX_SEGMENTS, Segment},
};
use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    ui::UiSystems,
};

#[derive(Clone, Debug, PartialEq, ShaderType)]
struct FrameUniform {
    segments: [Segment; MAX_SEGMENTS],
    metrics: Vec4,
}

impl Default for FrameUniform {
    fn default() -> Self {
        Self {
            segments: [Segment::default(); MAX_SEGMENTS],
            metrics: Vec4::ZERO,
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
struct FrameMaterial {
    #[uniform(0)]
    data: FrameUniform,
}

impl UiMaterial for FrameMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_composable_ui/rails.wesl".into()
    }
}

/// Last resolution failure, if any. Invalid edits keep the last valid paint.
/// An initially invalid frame is transparent. The plugin retries when anchors reappear.
#[derive(Component, Default, Debug)]
pub struct FrameDiagnostic(pub Option<FrameError>);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrameRegionsSystems;

pub struct FrameRegionsPlugin;
impl Plugin for FrameRegionsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "rails.wesl");
        app.add_plugins(UiMaterialPlugin::<FrameMaterial>::default())
            .add_systems(
                PostUpdate,
                attach_materials
                    .after(crate::ReactiveViewsSystems)
                    .before(UiSystems::Prepare),
            )
            .add_systems(
                PostUpdate,
                resolve_frames
                    .after(UiSystems::Layout)
                    .in_set(FrameRegionsSystems),
            );
    }
}

fn attach_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<FrameMaterial>>,
    added: Query<Entity, (With<SharedFrame>, Without<MaterialNode<FrameMaterial>>)>,
    mut removed: RemovedComponents<SharedFrame>,
    frames: Query<(), With<SharedFrame>>,
) {
    for entity in &added {
        commands.entity(entity).insert((
            MaterialNode(materials.add(FrameMaterial::default())),
            FrameDiagnostic::default(),
            Pickable::IGNORE,
        ));
    }
    for entity in removed.read() {
        if !frames.contains(entity)
            && let Ok(mut entity) = commands.get_entity(entity)
        {
            entity.remove::<(MaterialNode<FrameMaterial>, FrameDiagnostic)>();
        }
    }
}

fn resolve_frames(
    mut frames: Query<(
        &SharedFrame,
        &ComputedNode,
        &UiGlobalTransform,
        &MaterialNode<FrameMaterial>,
        &mut FrameDiagnostic,
    )>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut materials: ResMut<Assets<FrameMaterial>>,
) {
    for (frame, node, transform, handle, mut diagnostic) in &mut frames {
        if node.size().min_element() <= 0. {
            continue;
        }
        let Some(inverse) = transform.try_inverse() else {
            continue;
        };
        let scale = node.inverse_scale_factor();
        let geometry = frame.resolve(node.size() * scale, |entity| {
            let (child, transform) = nodes.get(entity).ok()?;
            let half = child.size() / 2.;
            let mut rect = Rect {
                min: Vec2::splat(f32::INFINITY),
                max: Vec2::splat(f32::NEG_INFINITY),
            };
            for corner in [
                Vec2::new(-half.x, -half.y),
                Vec2::new(half.x, -half.y),
                half,
                Vec2::new(-half.x, half.y),
            ] {
                let p = (inverse.transform_point2(transform.affine().transform_point2(corner))
                    + node.size() / 2.)
                    * scale;
                rect.min = rect.min.min(p);
                rect.max = rect.max.max(p);
            }
            Some(rect)
        });
        match geometry {
            Ok(geometry) => {
                if diagnostic.0.is_some() {
                    diagnostic.0 = None;
                }
                let mut data = FrameUniform {
                    metrics: Vec4::new(
                        scale,
                        geometry.segment_count() as f32,
                        f32::from(frame.guides),
                        frame.opacity,
                    ),
                    ..default()
                };
                data.segments[..geometry.segment_count()].copy_from_slice(&geometry.segments);
                if materials
                    .get(&handle.0)
                    .is_some_and(|material| material.data != data)
                    && let Some(mut material) = materials.get_mut(&handle.0)
                {
                    material.data = data;
                }
            }
            Err(error) => {
                if diagnostic.0.as_ref() != Some(&error) {
                    warn!(%error, "Shared frame resolution failed; retaining last valid geometry");
                    diagnostic.0 = Some(error);
                }
            }
        }
    }
}
