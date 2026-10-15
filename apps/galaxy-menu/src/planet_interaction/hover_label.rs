//! UI labels that track the screen position of the currently hovered planet.

use bevy::prelude::*;

use super::PlanetInteractionState;
use crate::PlanetMarker;

const LABEL_COLOR: Color = Color::srgb(1.0, 0.84, 0.62);
const LABEL_BACKGROUND: Color = Color::srgba(0.025, 0.035, 0.09, 0.9);

#[derive(Component, Clone, Copy, Debug)]
pub struct PlanetHoverLabel {
    pub planet_entity: Entity,
    pub planet_id: crate::PlanetId,
}

pub(super) fn setup_planet_hover_labels(
    mut commands: Commands,
    planets: Query<(Entity, &PlanetMarker)>,
) {
    for (planet_entity, planet) in &planets {
        let profile = planet.id.profile();
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                padding: UiRect::axes(px(10), px(6)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(LABEL_BACKGROUND),
            Text::new(profile.name),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(LABEL_COLOR),
            PlanetHoverLabel {
                planet_entity,
                planet_id: planet.id,
            },
            ZIndex(20),
        ));
    }
}

pub(super) fn position_hover_labels(
    state: Res<PlanetInteractionState>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    planets: Query<&GlobalTransform, With<PlanetMarker>>,
    mut labels: Query<(&PlanetHoverLabel, &mut Node)>,
) {
    let camera = cameras.iter().next();

    for (label, mut node) in &mut labels {
        if state.hovered_planet != Some(label.planet_id) {
            node.display = Display::None;
            continue;
        }

        node.display = Display::Flex;
        let Some((camera, camera_transform)) = camera else {
            continue;
        };
        let Ok(planet_transform) = planets.get(label.planet_entity) else {
            continue;
        };

        // Convert the planet's propagated world position into the UI's screen-space position.
        if let Ok(screen_position) =
            camera.world_to_viewport(camera_transform, planet_transform.translation())
        {
            node.left = px(screen_position.x + 16.0);
            node.top = px(screen_position.y + 12.0);
        } else {
            // The fallback keeps the label observable in headless app tests without a viewport.
            node.left = px(470);
            node.top = px(72);
        }
    }
}
