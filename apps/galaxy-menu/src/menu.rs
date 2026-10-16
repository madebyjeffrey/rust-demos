//! Sparse placeholder menu shown alongside the galaxy scene.

use bevy::prelude::*;

use crate::planet_interaction::BlocksPlanetInteraction;

const BACKGROUND: Color = Color::srgb(0.025, 0.035, 0.09);
const ACCENT: Color = Color::srgb(0.96, 0.66, 0.39);
const PRIMARY_TEXT: Color = Color::srgb(0.98, 0.83, 0.6);
const SECONDARY_TEXT: Color = Color::srgb(0.61, 0.68, 0.79);
const QUIET_TEXT: Color = Color::srgb(0.39, 0.47, 0.61);

/// Identifies a visual menu action; no action behavior is attached in this prototype.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuAction {
    NewGame,
    LoadGame,
    Settings,
    Quit,
}

/// Lets tests and presentation code distinguish the two levels of menu emphasis.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuEmphasis {
    Primary,
    Secondary,
}

/// Adds the menu shell and its four placeholder actions to the app.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_menu);
    }
}

fn setup_menu(mut commands: Commands) {
    commands.insert_resource(ClearColor(BACKGROUND));
    commands.spawn((
        BlocksPlanetInteraction,
        Node {
            position_type: PositionType::Absolute,
            left: px(54),
            top: px(48),
            width: px(380),
            flex_direction: FlexDirection::Column,
            row_gap: px(22),
            ..default()
        },
        children![
            (
                Node {
                    width: px(62),
                    height: px(7),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(ACCENT),
            ),
            (
                Text::new("GALAXY MENU"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(ACCENT),
            ),
            (
                Text::new("CHOOSE YOUR NEXT STEP"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(QUIET_TEXT),
                Node {
                    margin: UiRect::top(px(-13)),
                    ..default()
                },
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    margin: UiRect::top(px(14)),
                    ..default()
                },
                children![
                    action_text("New Game", MenuAction::NewGame, MenuEmphasis::Primary),
                    action_text("Load Game", MenuAction::LoadGame, MenuEmphasis::Primary),
                ],
            ),
            (Node {
                height: px(10),
                ..default()
            },),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                },
                children![
                    action_text("Settings", MenuAction::Settings, MenuEmphasis::Secondary),
                    action_text("Quit", MenuAction::Quit, MenuEmphasis::Secondary),
                ],
            ),
        ],
    ));
}

fn action_text(label: &'static str, action: MenuAction, emphasis: MenuEmphasis) -> impl Bundle {
    let (font_size, color) = match emphasis {
        MenuEmphasis::Primary => (FontSize::Px(32.0), PRIMARY_TEXT),
        MenuEmphasis::Secondary => (FontSize::Px(19.0), SECONDARY_TEXT),
    };

    (
        Text::new(label),
        TextFont {
            font_size,
            ..default()
        },
        TextColor(color),
        action,
        emphasis,
    )
}
