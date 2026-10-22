//! Layout-independent visual motion for keyed views. Bevy computes the target geometry;
//! corrections run before clipping, picking on the next frame, and frame-region resolution.

use crate::reactive::{Activation, ReactiveRoot};
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    math::Affine2,
    prelude::*,
    ui::{IgnoreScroll, InteractionDisabled, Pressed, UiSystems},
};
use std::{collections::HashMap, time::Duration};

/// Slide/scale insertion and removal, with smooth position interpolation for reordering.
/// Offsets are logical pixels in the parent's coordinates. Scale must be finite and positive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewTransition {
    pub duration: Duration,
    pub enter_offset: Vec2,
    pub exit_offset: Vec2,
    pub edge_scale: f32,
}
impl Default for ViewTransition {
    fn default() -> Self {
        Self {
            duration: Duration::from_millis(280),
            enter_offset: Vec2::new(0., 12.),
            exit_offset: Vec2::new(0., -12.),
            edge_scale: 0.65,
        }
    }
}
impl ViewTransition {
    pub(crate) fn valid(&self) -> bool {
        self.enter_offset.is_finite()
            && self.exit_offset.is_finite()
            && self.edge_scale.is_finite()
            && self.edge_scale > 0.
    }
}

/// Applications can disable motion for reduced-motion preferences. Exits then finish immediately.
#[derive(Resource, Debug)]
pub struct TransitionSettings {
    pub enabled: bool,
}
impl Default for TransitionSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionPhase {
    Entering,
    Moving,
    Exiting,
    Idle,
}

/// Read-only transition telemetry. Progress is linear time in [0, 1]; motion uses smoothstep.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TransitionStatus {
    pub phase: TransitionPhase,
    pub progress: f32,
}

/// A removed view retained only for exit paint. Also present on its descendants.
/// These entities are excluded from bindings, activation, picking and sequential focus.
#[derive(Component)]
pub struct ExitingView;

#[derive(Component, Clone, Copy, PartialEq)]
struct Motion {
    spec: ViewTransition,
    target: Option<Affine2>,
    parent_size: Vec2,
    parent_scroll: Vec2,
    offset: Vec2,
    scale: f32,
    from_offset: Vec2,
    from_scale: f32,
    elapsed: f32,
    phase: TransitionPhase,
    relayout: u8,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewTransitionSystems;

pub(crate) fn install(app: &mut App) {
    app.init_resource::<TransitionSettings>().add_systems(
        PostUpdate,
        animate
            .after(UiSystems::Layout)
            .before(UiSystems::PostLayout)
            .in_set(ViewTransitionSystems),
    );
}

pub(crate) fn configure(world: &mut World, entity: Entity, spec: Option<ViewTransition>) {
    if let Some(spec) = spec {
        if spec.duration.is_zero() {
            world
                .entity_mut(entity)
                .remove::<Motion>()
                .insert(TransitionStatus {
                    phase: TransitionPhase::Idle,
                    progress: 1.,
                });
            return;
        }
        if let Some(mut motion) = world.get_mut::<Motion>(entity) {
            motion.spec = spec;
        } else {
            world.entity_mut(entity).insert((
                Motion {
                    spec,
                    target: None,
                    parent_size: Vec2::ZERO,
                    parent_scroll: Vec2::ZERO,
                    offset: Vec2::ZERO,
                    scale: 1.,
                    from_offset: Vec2::ZERO,
                    from_scale: 1.,
                    elapsed: 0.,
                    phase: TransitionPhase::Entering,
                    relayout: 0,
                },
                TransitionStatus {
                    phase: TransitionPhase::Entering,
                    progress: 0.,
                },
            ));
        }
    } else {
        world
            .entity_mut(entity)
            .remove::<(Motion, TransitionStatus)>();
    }
}

pub(crate) fn relayout(world: &mut World, entity: Entity) {
    if let Some(mut motion) = world.get_mut::<Motion>(entity) {
        // Window target propagation and text measurement may finish a frame later.
        motion.relayout = 2;
    }
}

pub(crate) fn retire(world: &mut World, entity: Entity) -> bool {
    if !world
        .get_resource::<TransitionSettings>()
        .is_some_and(|settings| settings.enabled)
    {
        return false;
    }
    let Some(motion) = world.get::<Motion>(entity) else {
        return false;
    };
    if motion.target.is_none() || motion.spec.duration.is_zero() {
        return false;
    }
    let size = world
        .get::<ComputedNode>(entity)
        .map(|n| n.size() * n.inverse_scale_factor());
    let Some(size) = size else { return false };
    let mut node = world.get::<Node>(entity).cloned().unwrap_or_default();
    // Retain the same hierarchy for paint, but release its allocation immediately.
    node.position_type = PositionType::Absolute;
    node.left = px(0);
    node.top = px(0);
    node.right = Val::Auto;
    node.bottom = Val::Auto;
    node.width = px(size.x);
    node.height = px(size.y);
    node.min_width = px(size.x);
    node.min_height = px(size.y);
    node.max_width = px(size.x);
    node.max_height = px(size.y);
    node.margin = UiRect::ZERO;
    world.entity_mut(entity).insert(node);
    let mut motion = world.get_mut::<Motion>(entity).unwrap();
    motion.phase = TransitionPhase::Exiting;
    motion.elapsed = 0.;
    motion.from_offset = motion.offset;
    motion.from_scale = motion.scale;
    motion.relayout = 0;
    world.entity_mut(entity).insert(TransitionStatus {
        phase: TransitionPhase::Exiting,
        progress: 0.,
    });
    for descendant in subtree(world, entity) {
        world
            .entity_mut(descendant)
            .insert((ExitingView, Pickable::IGNORE, InteractionDisabled))
            .remove::<(TabIndex, TabGroup, Pressed, Activation, ReactiveRoot)>();
        // A parent's removal owns the entire exit. Nested transitions must not keep ticking.
        if descendant != entity {
            world
                .entity_mut(descendant)
                .remove::<(Motion, TransitionStatus)>();
        }
    }
    true
}

fn subtree(world: &World, root: Entity) -> Vec<Entity> {
    let mut nodes = vec![root];
    let mut index = 0;
    while index < nodes.len() {
        if let Some(children) = world.get::<Children>(nodes[index]) {
            nodes.extend(children.iter());
        }
        index += 1;
    }
    nodes
}

fn paint_subtree(world: &World, root: Entity) -> Vec<Entity> {
    let mut nodes = vec![root];
    let mut index = 0;
    while index < nodes.len() {
        if let Some(children) = world.get::<Children>(nodes[index]) {
            nodes.extend(children.iter().filter(|entity| {
                world.get::<FixedNode>(*entity).is_none()
                    && world.get::<UiTargetCamera>(*entity).is_none()
            }));
        }
        index += 1;
    }
    nodes
}

fn depth(world: &World, mut entity: Entity) -> usize {
    let mut depth = 0;
    while let Some(parent) = world.get::<ChildOf>(entity) {
        depth += 1;
        entity = parent.parent();
    }
    depth
}

fn animate(world: &mut World) {
    let enabled = world.resource::<TransitionSettings>().enabled;
    let dt = world.get_resource::<Time>().map_or(0., Time::delta_secs);
    let mut roots: Vec<_> = world
        .query_filtered::<Entity, With<Motion>>()
        .iter(world)
        .collect();
    if roots.is_empty() {
        return;
    }
    roots.sort_by_key(|entity| depth(world, *entity));
    // Snapshot the native layout before applying any visual corrections. Nested motion then
    // composes with an ancestor's corrected transform without changing the target layout.
    let native: HashMap<_, _> = roots
        .iter()
        .flat_map(|entity| {
            [
                Some(*entity),
                world.get::<ChildOf>(*entity).map(ChildOf::parent),
            ]
        })
        .flatten()
        .filter_map(|entity| {
            world
                .get::<UiGlobalTransform>(entity)
                .map(|transform| (entity, transform.affine()))
        })
        .collect();
    for entity in roots {
        let Some(mut motion) = world.get::<Motion>(entity).copied() else {
            continue;
        };
        if motion.phase == TransitionPhase::Exiting {
            motion.elapsed += dt;
            if !enabled || motion.elapsed >= motion.spec.duration.as_secs_f32() {
                world.despawn(entity);
                continue;
            }
        }
        if motion.target.is_none()
            && world
                .get::<ComputedNode>(entity)
                .is_none_or(|node| node.is_empty())
        {
            world.entity_mut(entity).insert(motion);
            continue;
        }
        let Some(base) = native.get(&entity).copied() else {
            world.entity_mut(entity).insert(motion);
            continue;
        };
        let parent = world.get::<ChildOf>(entity).map(ChildOf::parent);
        let parent_native = parent
            .and_then(|e| native.get(&e).copied())
            .unwrap_or(Affine2::IDENTITY);
        if parent_native.matrix2.determinant() == 0. || base.matrix2.determinant() == 0. {
            world.entity_mut(entity).insert(motion);
            continue;
        }
        let dpi = world
            .get::<ComputedNode>(entity)
            .map_or(1., |node| node.inverse_scale_factor().recip());
        let (parent_size, scroll) = parent
            .and_then(|e| world.get::<ComputedNode>(e))
            .map_or((Vec2::ZERO, Vec2::ZERO), |node| {
                (node.size(), node.scroll_position)
            });
        let parent_scroll = world
            .get::<IgnoreScroll>(entity)
            .map_or(scroll, |ignore| scroll * Vec2::from(!ignore.0));
        // UI transforms use a centered parent origin. Compare positions in the parent's
        // content coordinates so resizing or scrolling does not masquerade as reordering.
        let rebase =
            (motion.parent_size - parent_size) * 0.5 + motion.parent_scroll - parent_scroll;
        let target = if motion.phase == TransitionPhase::Exiting {
            let mut frozen = motion.target.unwrap();
            frozen.translation += rebase;
            frozen
        } else {
            parent_native.inverse() * base
        };
        let mut restarted = false;
        if let Some(previous) = motion.target {
            if (motion.relayout > 0
                || matches!(
                    motion.phase,
                    TransitionPhase::Entering | TransitionPhase::Moving
                ))
                && motion.phase != TransitionPhase::Exiting
                && (previous.translation + rebase - target.translation).length_squared() > 0.01
            {
                motion.from_offset =
                    motion.offset + previous.translation + rebase - target.translation;
                motion.from_scale = motion.scale;
                if motion.phase != TransitionPhase::Entering {
                    motion.phase = TransitionPhase::Moving;
                }
                motion.elapsed = 0.;
                restarted = true;
            }
        } else {
            motion.from_offset = motion.spec.enter_offset * dpi;
            motion.from_scale = motion.spec.edge_scale;
            motion.elapsed = 0.;
            restarted = true;
        }
        motion.relayout = motion.relayout.saturating_sub(1);
        motion.target = Some(target);
        motion.parent_size = parent_size;
        motion.parent_scroll = parent_scroll;
        if !restarted
            && !matches!(
                motion.phase,
                TransitionPhase::Exiting | TransitionPhase::Idle
            )
        {
            motion.elapsed += dt;
        }
        let progress = if motion.phase == TransitionPhase::Idle
            || !enabled
            || motion.spec.duration.is_zero()
        {
            1.
        } else {
            (motion.elapsed / motion.spec.duration.as_secs_f32()).min(1.)
        };
        if motion.phase == TransitionPhase::Exiting && progress >= 1. {
            world.despawn(entity);
            continue;
        }
        let t = progress * progress * (3. - 2. * progress);
        let (end_offset, end_scale) = if motion.phase == TransitionPhase::Exiting {
            (motion.spec.exit_offset * dpi, motion.spec.edge_scale)
        } else {
            (Vec2::ZERO, 1.)
        };
        motion.offset = motion.from_offset.lerp(end_offset, t);
        motion.scale = motion.from_scale + (end_scale - motion.from_scale) * t;
        if progress >= 1. {
            motion.phase = TransitionPhase::Idle;
        }
        let visual_local = Affine2::from_mat2_translation(
            target.matrix2 * motion.scale,
            target.translation + motion.offset,
        );
        let parent_visual = parent
            .and_then(|e| world.get::<UiGlobalTransform>(e))
            .map_or(Affine2::IDENTITY, UiGlobalTransform::affine);
        if motion.offset != Vec2::ZERO
            || motion.scale != 1.
            || motion.phase == TransitionPhase::Exiting
        {
            let current = world.get::<UiGlobalTransform>(entity).unwrap().affine();
            if current.matrix2.determinant() != 0. {
                let correction = parent_visual * visual_local * current.inverse();
                for descendant in paint_subtree(world, entity) {
                    if let Some(mut transform) = world.get_mut::<UiGlobalTransform>(descendant) {
                        *transform = (correction * transform.affine()).into();
                    }
                }
            }
        }
        let status = TransitionStatus {
            phase: motion.phase,
            progress,
        };
        if world.get::<TransitionStatus>(entity) != Some(&status) {
            world.entity_mut(entity).insert(status);
        }
        if world.get::<Motion>(entity) != Some(&motion) {
            world.entity_mut(entity).insert(motion);
        }
    }
}
