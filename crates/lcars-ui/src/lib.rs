#![doc = include_str!("../README.md")]

//! Composable LCARS screens built from native Bevy UI entities.
//!
//! Install [`LcarsPlugin`] alongside Bevy's `DefaultPlugins`, spawn your camera,
//! then build a [`Screen`]. Application state stays in your own resources:
//! observe Bevy's `Activate` / `ValueChange<bool>` events and synchronize the
//! widgets' `Checked` components in `Update`. Presentation runs in `PostUpdate`.
//! See the crate README for a complete example and the supported layout contract.
mod builder;
mod frame;
mod presentation;

pub use builder::{TextStyle, Ui, Widget};
pub use frame::{Screen, ScreenEntities};

use bevy::{input_focus::tab_navigation::TabNavigationPlugin, prelude::*, ui::UiSystems};

/// Adds keyboard navigation, automatic styling, progress rendering, and screen fitting.
/// Requires Bevy's UI and picking plugins (included by `DefaultPlugins` with `ui`).
pub struct LcarsPlugin;
impl Plugin for LcarsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        app.add_systems(
            PostUpdate,
            (
                presentation::style_widgets,
                presentation::update_progress,
                frame::fit_screens,
            )
                .before(UiSystems::Prepare),
        );
    }
}

/// The shared LCARS palette. Use a tone consistently for a related set of controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Orange,
    Peach,
    Lilac,
    Blue,
}
impl Tone {
    pub const fn color(self) -> Color {
        match self {
            Self::Orange => Color::srgb(1.0, 0.65, 0.32),
            Self::Peach => Color::srgb(1.0, 0.79, 0.60),
            Self::Lilac => Color::srgb(0.70, 0.64, 0.88),
            Self::Blue => Color::srgb(0.55, 0.73, 0.94),
        }
    }
}

pub const INK: Color = Color::srgb(0.025, 0.025, 0.04);
pub const PANEL: Color = Color::srgb(0.065, 0.065, 0.10);
const MUTED: Color = Color::srgb(0.48, 0.49, 0.60);
const WHITE: Color = Color::srgb(0.95, 0.91, 0.86);

/// Normalized progress on the entity returned by [`Ui::progress`].
/// Rendering clamps to 0..=1; non-finite values render as zero.
#[derive(Component, Default, Debug)]
pub struct Progress(pub f32);

#[derive(Component)]
struct ProgressFill;
#[derive(Component)]
struct ControlStyle {
    tone: Tone,
    primary: bool,
}
#[derive(Component)]
struct SelectionMark;
