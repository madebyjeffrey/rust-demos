//! Faint stellar companions on independent, inclined orbits around the main galaxy.

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    picking::Pickable,
    prelude::*,
};

use crate::{
    galaxy_map::GalaxyRoot,
    starfield::{StableRandom, StarMaterial, StarMeshBuilder},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalaxyCompanionKind {
    DwarfGalaxy,
    GlobularCluster,
}

/// Decorative stellar systems, separate from the selectable destination markers.
#[derive(Component, Debug)]
pub struct GalaxyCompanion {
    pub kind: GalaxyCompanionKind,
    pub(super) framing_radius: f32,
}

#[derive(Component)]
pub(super) struct CompanionOrbit {
    axes: Vec2,
    plane: Quat,
    phase: f32,
    angular_speed: f32,
}

impl CompanionOrbit {
    fn offset(&self) -> Vec3 {
        self.plane
            * Vec3::new(
                self.axes.x * self.phase.cos(),
                0.0,
                self.axes.y * self.phase.sin(),
            )
    }
}

struct CompanionProfile {
    kind: GalaxyCompanionKind,
    seed: u64,
    star_count: usize,
    core_radius: f32,
    outer_radius: f32,
    shape: Vec3,
    axes: Vec2,
    inclination: f32,
    node: f32,
    phase: f32,
    angular_speed: f32,
}

pub(super) fn spawn_galaxy_companions(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StarMaterial>,
    center: Vec3,
) {
    let profiles = [
        CompanionProfile {
            kind: GalaxyCompanionKind::DwarfGalaxy,
            seed: 0xD0AF_2026_0001,
            star_count: 3_200,
            core_radius: 0.55,
            outer_radius: 2.1,
            shape: Vec3::new(1.0, 0.65, 0.8),
            axes: Vec2::new(15.5, 14.0),
            inclination: 0.85,
            node: 0.3,
            phase: 3.9,
            angular_speed: 0.006,
        },
        CompanionProfile {
            kind: GalaxyCompanionKind::DwarfGalaxy,
            seed: 0xD0AF_2026_0002,
            star_count: 2_400,
            core_radius: 0.45,
            outer_radius: 1.7,
            shape: Vec3::new(0.8, 0.7, 1.0),
            axes: Vec2::new(16.0, 14.0),
            inclination: -0.65,
            node: 2.2,
            phase: 4.0,
            angular_speed: -0.0045,
        },
        CompanionProfile {
            kind: GalaxyCompanionKind::GlobularCluster,
            seed: 0xC105_2026_0001,
            star_count: 1_100,
            core_radius: 0.13,
            outer_radius: 0.65,
            shape: Vec3::ONE,
            axes: Vec2::new(13.8, 13.0),
            inclination: 1.1,
            node: 1.0,
            phase: 2.5,
            angular_speed: 0.011,
        },
        CompanionProfile {
            kind: GalaxyCompanionKind::GlobularCluster,
            seed: 0xC105_2026_0002,
            star_count: 850,
            core_radius: 0.10,
            outer_radius: 0.55,
            shape: Vec3::ONE,
            axes: Vec2::new(14.8, 13.4),
            inclination: -1.25,
            node: -0.7,
            phase: 0.8,
            angular_speed: -0.009,
        },
        CompanionProfile {
            kind: GalaxyCompanionKind::GlobularCluster,
            seed: 0xC105_2026_0003,
            star_count: 1_250,
            core_radius: 0.15,
            outer_radius: 0.8,
            shape: Vec3::ONE,
            axes: Vec2::new(16.3, 14.8),
            inclination: 0.95,
            node: -1.6,
            phase: 5.2,
            angular_speed: 0.008,
        },
    ];

    for profile in profiles {
        let orbit = CompanionOrbit {
            axes: profile.axes,
            plane: Quat::from_rotation_y(profile.node) * Quat::from_rotation_x(profile.inclination),
            phase: profile.phase,
            angular_speed: profile.angular_speed,
        };
        let transform = Transform::from_translation(center + orbit.offset())
            .with_rotation(Quat::from_euler(EulerRot::YXZ, profile.node, 0.4, 0.25));
        commands.spawn((
            GalaxyCompanion {
                kind: profile.kind,
                framing_radius: profile.outer_radius,
            },
            orbit,
            Mesh3d(meshes.add(build_companion_stars(&profile))),
            MeshMaterial3d(material.clone()),
            transform,
            Pickable::IGNORE,
        ));
    }
}

fn build_companion_stars(profile: &CompanionProfile) -> Mesh {
    let mut rng = StableRandom::new(profile.seed);
    let mut mesh = StarMeshBuilder::with_capacity(profile.star_count);
    for _ in 0..profile.star_count {
        // A concentrated core with a sparse, smoothly fading stellar envelope.
        let radius = loop {
            let u = rng.unit().clamp(0.0001, 0.9999);
            let radius = profile.core_radius / (u.powf(-2.0 / 3.0) - 1.0).sqrt();
            if radius < profile.outer_radius {
                break radius;
            }
        };
        let direction = Vec3::new(rng.normal(), rng.normal(), rng.normal()).normalize_or_zero();
        let point = direction * radius * profile.shape;
        let edge = ((1.0 - radius / profile.outer_radius) / 0.4).clamp(0.0, 1.0);
        let fade = edge * edge * (3.0 - 2.0 * edge);
        let (tint, size, brightness) = match profile.kind {
            GalaxyCompanionKind::DwarfGalaxy => (
                [0.74, 0.84, 1.0],
                0.005 + rng.unit().powi(2) * 0.009,
                0.3 + rng.unit() * 0.65,
            ),
            GalaxyCompanionKind::GlobularCluster => (
                [1.0, 0.86, 0.67],
                0.005 + rng.unit().powi(2) * 0.008,
                0.5 + rng.unit() * 0.9,
            ),
        };
        mesh.push_star(
            point,
            size,
            [
                tint[0] * brightness * fade,
                tint[1] * brightness * fade,
                tint[2] * brightness * fade,
                1.0,
            ],
        );
    }
    mesh.build()
}

pub(super) fn update_companion_orbits(
    mut companions: Query<(&mut Transform, &mut CompanionOrbit), Without<GalaxyRoot>>,
    roots: Query<&Transform, With<GalaxyRoot>>,
    time: Res<Time>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    if mouse_buttons.pressed(MouseButton::Left)
        || mouse_motion.delta != Vec2::ZERO
        || mouse_scroll.delta != Vec2::ZERO
    {
        return;
    }
    let Ok(root) = roots.single() else {
        return;
    };
    for (mut transform, mut orbit) in &mut companions {
        orbit.phase = (orbit.phase + orbit.angular_speed * time.delta_secs())
            .rem_euclid(std::f32::consts::TAU);
        transform.translation = root.translation + orbit.offset();
    }
}
