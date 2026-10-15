//! Destination-card layout and the projection from selection state to card content.

use bevy::prelude::*;

use super::PlanetCardState;
use crate::PlanetId;

const BACKGROUND: Color = Color::srgba(0.025, 0.035, 0.09, 0.9);
const PRIMARY_TEXT: Color = Color::srgb(0.98, 0.83, 0.6);
const SECONDARY_TEXT: Color = Color::srgb(0.72, 0.77, 0.85);
const QUIET_TEXT: Color = Color::srgb(0.61, 0.68, 0.79);

#[derive(Component, Debug)]
pub struct PlanetCardName;

#[derive(Component, Debug)]
pub struct PlanetCardDetail;

#[derive(Component, Debug)]
pub(super) struct PlanetCardVisual;

pub(super) fn setup_planet_card(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(42),
                top: px(42),
                width: px(286),
                padding: UiRect::all(px(18)),
                flex_direction: FlexDirection::Column,
                row_gap: px(9),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(BACKGROUND),
            ZIndex(20),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new("DESTINATION"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(QUIET_TEXT),
            ));
            card.spawn((
                Node {
                    align_self: AlignSelf::Center,
                    width: px(96),
                    height: px(96),
                    margin: UiRect::vertical(px(4)),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(48)),
                    ..default()
                },
                BackgroundColor(PlanetId::Vesper.profile().card_color),
                BorderColor::all(Color::srgba(1.0, 0.9, 0.76, 0.62)),
                BoxShadow::new(
                    Color::srgba(0.35, 0.74, 1.0, 0.42),
                    px(0),
                    px(0),
                    px(3),
                    px(22),
                ),
                PlanetCardVisual,
            ))
            .with_children(|planet| {
                planet.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(19),
                        top: px(15),
                        width: px(22),
                        height: px(16),
                        border_radius: BorderRadius::all(px(11)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.48)),
                ));
            });
            card.spawn((
                Text::new("Select a planet"),
                TextFont {
                    font_size: FontSize::Px(25.0),
                    ..default()
                },
                TextColor(PRIMARY_TEXT),
                PlanetCardName,
            ));
            card.spawn((
                Text::new("Hover over a planet for a closer look."),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(SECONDARY_TEXT),
                PlanetCardDetail,
            ));
        });
}

pub(super) fn update_planet_card(
    card: Res<PlanetCardState>,
    mut names: Query<&mut Text, With<PlanetCardName>>,
    mut details: Query<&mut Text, (With<PlanetCardDetail>, Without<PlanetCardName>)>,
    mut visuals: Query<&mut BackgroundColor, With<PlanetCardVisual>>,
) {
    let profile = card.planet_id.map(PlanetId::profile);
    for mut text in &mut names {
        text.0 = profile
            .map_or("Select a planet", |profile| profile.name)
            .to_owned();
    }
    for mut text in &mut details {
        text.0 = profile
            .map_or("Hover over a planet for a closer look.", |profile| {
                profile.detail
            })
            .to_owned();
    }
    for mut background in &mut visuals {
        background.0 = profile.map_or(PRIMARY_TEXT, |profile| profile.card_color);
    }
}
