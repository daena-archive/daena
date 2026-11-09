//! Named starting points for [`WorldSettings`].

use crate::{ContinentLayout, PlanetSettings, WorldSettings};

/// Named starting points. Each sets physical parameters only; every field stays editable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Preset {
    /// Earth's forcing and the default world.
    Earthlike,
    /// Weak greenhouse, sea ice to mid-latitudes.
    Frozen,
    /// Strong greenhouse, no polar ice.
    Hothouse,
    /// Much land and little evaporative supply.
    Desert,
    /// Little land on many small, active plates.
    Archipelago,
    /// Islands in a global ocean.
    WaterWorld,
    /// One supercontinent on few plates.
    Pangea,
    /// Many plates at high activity.
    Shattered,
}

impl Preset {
    /// Every preset, in display order.
    pub const ALL: [Preset; 8] = [
        Preset::Earthlike,
        Preset::Frozen,
        Preset::Hothouse,
        Preset::Desert,
        Preset::Archipelago,
        Preset::WaterWorld,
        Preset::Pangea,
        Preset::Shattered,
    ];

    /// Stable name used by the CLI and the JSON report.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Earthlike => "earthlike",
            Self::Frozen => "frozen",
            Self::Hothouse => "hothouse",
            Self::Desert => "desert",
            Self::Archipelago => "archipelago",
            Self::WaterWorld => "water-world",
            Self::Pangea => "pangea",
            Self::Shattered => "shattered",
        }
    }

    /// Parse one of the names from [`Self::as_str`].
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "earthlike" => Some(Self::Earthlike),
            "frozen" => Some(Self::Frozen),
            "hothouse" => Some(Self::Hothouse),
            "desert" => Some(Self::Desert),
            "archipelago" => Some(Self::Archipelago),
            "water-world" => Some(Self::WaterWorld),
            "pangea" => Some(Self::Pangea),
            "shattered" => Some(Self::Shattered),
            _ => None,
        }
    }

    /// One line for `--list-presets`.
    pub fn description(self) -> &'static str {
        match self {
            Self::Earthlike => "Earth's forcing, 29 % land on 14 plates.",
            Self::Frozen => {
                "Weak greenhouse: sea ice to mid-latitudes, ice sheets and a cool equator."
            }
            Self::Hothouse => {
                "Strong greenhouse: no polar ice, about twice Earth's rain and high seas."
            }
            Self::Desert => {
                "60 % land on a water-poor planet: dry interiors, salt lakes and closed basins."
            }
            Self::Archipelago => "10 % land on 30 small, active plates: island arcs in warm seas.",
            Self::WaterWorld => "2 % land: scattered islands in a warm global ocean.",
            Self::Pangea => "One supercontinent on 8 large plates.",
            Self::Shattered => {
                "40 plates at high activity: many small landmasses, straits and islands."
            }
        }
    }

    /// Starting settings. Seed and resolution stay at their defaults; callers set them.
    ///
    /// The climate presets use the greenhouse knob, not `temperature_offset_c`, so the
    /// artistic offset stays free for the user. Frozen stays mild: with ice-sheet albedo
    /// and weaker moist transport in cold air, −12 W/m² already slides toward a snowball.
    pub fn settings(self) -> WorldSettings {
        let base = WorldSettings::default();
        let (
            land_fraction,
            plate_count,
            continent_layout,
            tectonic_activity,
            greenhouse_wm2,
            precipitation_scale,
        ) = match self {
            Self::Earthlike => (0.29, 14, ContinentLayout::Mixed, 1.0, 0.0, 1.0),
            Self::Frozen => (0.32, 14, ContinentLayout::Mixed, 1.0, -8.0, 1.0),
            Self::Hothouse => (0.25, 14, ContinentLayout::Mixed, 1.0, 20.0, 1.0),
            Self::Desert => (0.60, 14, ContinentLayout::Few, 1.0, 6.0, 0.25),
            Self::Archipelago => (0.10, 30, ContinentLayout::Many, 1.5, 8.0, 1.0),
            Self::WaterWorld => (0.02, 20, ContinentLayout::Many, 1.2, 8.0, 1.0),
            Self::Pangea => (0.30, 8, ContinentLayout::Pangea, 1.0, 0.0, 1.0),
            Self::Shattered => (0.35, 40, ContinentLayout::Many, 1.8, 0.0, 1.0),
        };
        WorldSettings {
            land_fraction,
            plate_count,
            continent_layout,
            tectonic_activity,
            planet: PlanetSettings {
                greenhouse_wm2,
                precipitation_scale,
                ..base.planet
            },
            ..base
        }
    }
}
