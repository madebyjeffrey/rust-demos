//! Planet identities and the display metadata shared by map and interaction systems.

use bevy::prelude::*;

/// Identity for a selectable destination on the galaxy map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlanetId {
    Vesper,
    Kestrel,
    Solace,
    Meridian,
    Halcyon,
}

/// The name, short detail, and destination-card appearance associated with a planet.
#[derive(Clone, Copy, Debug)]
pub struct PlanetProfile {
    pub name: &'static str,
    pub detail: &'static str,
    pub card_color: Color,
}

impl PlanetId {
    /// Returns the single source of display metadata for this destination.
    pub const fn profile(self) -> PlanetProfile {
        match self {
            Self::Vesper => PlanetProfile {
                name: "Vesper",
                detail: "Ocean world at the edge of the inner arm.",
                card_color: Color::srgb(0.35, 0.74, 1.0),
            },
            Self::Kestrel => PlanetProfile {
                name: "Kestrel",
                detail: "Wind carved deserts beneath a copper sky.",
                card_color: Color::srgb(1.0, 0.48, 0.29),
            },
            Self::Solace => PlanetProfile {
                name: "Solace",
                detail: "A quiet world lit by twin auroras.",
                card_color: Color::srgb(0.74, 0.52, 1.0),
            },
            Self::Meridian => PlanetProfile {
                name: "Meridian",
                detail: "A warm trade world at the center of the lanes.",
                card_color: Color::srgb(1.0, 0.70, 0.32),
            },
            Self::Halcyon => PlanetProfile {
                name: "Halcyon",
                detail: "A lush canopy world with a calm ocean climate.",
                card_color: Color::srgb(0.30, 0.85, 0.76),
            },
        }
    }
}

/// A selectable destination represented by a simple glowing planet marker.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanetMarker {
    pub id: PlanetId,
}
