use crate::grid::NONE;
use crate::{generate, NoProgress, Preset, World, WorldSettings};

fn build(preset: Preset) -> World {
    generate(
        &WorldSettings {
            seed: 7,
            resolution: 64,
            ..preset.settings()
        },
        &mut NoProgress,
    )
    .unwrap()
}

/// Area in square metres of each land component with at least four cells.
fn landmass_areas(world: &World) -> Vec<f64> {
    let n = world.grid.cell_count();
    let mut seen = vec![false; n];
    let mut areas = Vec::new();
    for start in 0..n {
        if seen[start] || world.ocean[start] != 0 {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![start as u32];
        let mut cells = 0usize;
        let mut area = 0.0;
        while let Some(cell) = stack.pop() {
            cells += 1;
            area += world.grid.area_m2(cell);
            for nb in world.grid.neighbors(cell) {
                if nb == NONE || seen[nb as usize] || world.ocean[nb as usize] != 0 {
                    continue;
                }
                seen[nb as usize] = true;
                stack.push(nb);
            }
        }
        if cells >= 4 {
            areas.push(area);
        }
    }
    areas
}

/// Share of all land area held by the largest landmass.
fn largest_share(world: &World) -> f64 {
    let land: f64 = (0..world.grid.cell_count())
        .filter(|&cell| world.ocean[cell] == 0)
        .map(|cell| world.grid.area_m2(cell as u32))
        .sum();
    let largest = landmass_areas(world).into_iter().fold(0.0, f64::max);
    largest / land
}

fn ocean_area(world: &World) -> f64 {
    (0..world.grid.cell_count())
        .filter(|&cell| world.ocean[cell] == 1)
        .map(|cell| world.grid.area_m2(cell as u32))
        .sum()
}

/// Area of ocean cells with sea ice in at least eleven months of the year.
fn perennial_ice_area(world: &World) -> f64 {
    (0..world.grid.cell_count())
        .filter(|&cell| world.ocean[cell] == 1 && world.climate.sea_ice_months[cell] >= 11)
        .map(|cell| world.grid.area_m2(cell as u32))
        .sum()
}

/// Area of ocean cells with sea ice in four to ten months of the year.
fn seasonal_ice_area(world: &World) -> f64 {
    (0..world.grid.cell_count())
        .filter(|&cell| {
            world.ocean[cell] == 1 && (4..=10).contains(&world.climate.sea_ice_months[cell])
        })
        .map(|cell| world.grid.area_m2(cell as u32))
        .sum()
}

#[test]
fn names_and_validation() {
    for preset in Preset::ALL {
        assert!(preset.settings().validate().is_ok(), "{preset:?}");
        assert_eq!(Preset::parse(preset.as_str()), Some(preset));
    }
    let names: Vec<&str> = Preset::ALL.iter().map(|preset| preset.as_str()).collect();
    for (i, name) in names.iter().enumerate() {
        assert!(!names[..i].contains(name), "duplicate name {name}");
    }
    assert_eq!(Preset::parse("Earthlike"), None);
    assert_eq!(Preset::parse(""), None);
    assert_eq!(Preset::parse("water_world"), None);
}

#[test]
fn earthlike_is_the_default_world() {
    let preset = Preset::Earthlike.settings();
    let default = WorldSettings::default();
    assert_eq!(preset.seed, default.seed);
    assert_eq!(preset.resolution, default.resolution);
    assert_eq!(preset.radius_m, default.radius_m);
    assert_eq!(preset.land_fraction, default.land_fraction);
    assert_eq!(preset.plate_count, default.plate_count);
    assert_eq!(preset.continent_layout, default.continent_layout);
    assert_eq!(preset.tectonic_activity, default.tectonic_activity);
    assert_eq!(preset.river_min_m3s, default.river_min_m3s);
    let (p, d) = (preset.planet, default.planet);
    assert_eq!(p.star_luminosity, d.star_luminosity);
    assert_eq!(p.star_mass, d.star_mass);
    assert_eq!(p.orbit_au, d.orbit_au);
    assert_eq!(p.eccentricity, d.eccentricity);
    assert_eq!(p.perihelion_deg, d.perihelion_deg);
    assert_eq!(p.axial_tilt_deg, d.axial_tilt_deg);
    assert_eq!(p.rotation_hours, d.rotation_hours);
    assert_eq!(p.greenhouse_wm2, d.greenhouse_wm2);
    assert_eq!(p.temperature_offset_c, d.temperature_offset_c);
    assert_eq!(p.precipitation_scale, d.precipitation_scale);
}

#[test]
fn presets_behave_as_named_at_n64() {
    let earth = build(Preset::Earthlike);
    let frozen = build(Preset::Frozen);
    let hothouse = build(Preset::Hothouse);
    let desert = build(Preset::Desert);
    let archipelago = build(Preset::Archipelago);
    let water = build(Preset::WaterWorld);
    let pangea = build(Preset::Pangea);
    let shattered = build(Preset::Shattered);

    let earth_t = earth.climate.stats.t_global_c;
    let frozen_t = frozen.climate.stats.t_global_c;
    assert!(
        frozen_t < earth_t - 8.0,
        "frozen global {frozen_t} earthlike {earth_t}"
    );
    let frozen_eq = frozen.climate.stats.t_equator_c;
    assert!(frozen_eq > 5.0, "frozen equator {frozen_eq}");
    let frozen_ice = perennial_ice_area(&frozen);
    let earth_ice = perennial_ice_area(&earth);
    assert!(
        frozen_ice > 1.5 * earth_ice,
        "frozen perennial ice {frozen_ice} earthlike {earth_ice}"
    );

    let hot_t = hothouse.climate.stats.t_global_c;
    assert!(
        hot_t > earth_t + 8.0,
        "hothouse global {hot_t} earthlike {earth_t}"
    );
    let hot_ice = perennial_ice_area(&hothouse) / ocean_area(&hothouse);
    assert!(hot_ice < 0.02, "hothouse perennial ice share {hot_ice}");
    // Moist transport: warm air carries more latent heat, so a hothouse has a
    // flatter pole-to-equator contrast. With fixed diffusivity it was 35.6 K.
    let hot_stats = &hothouse.climate.stats;
    let hot_gradient = hot_stats.t_equator_c - hot_stats.t_polar_c;
    assert!(hot_gradient < 31.0, "hothouse gradient {hot_gradient}");
    // Insulating sea ice: Earth-like ice is largely seasonal. When the air over
    // ice kept the full ocean heat capacity, seasonal ice was 4 % of perennial.
    let seasonal = seasonal_ice_area(&earth);
    let perennial = perennial_ice_area(&earth);
    assert!(
        seasonal > 0.5 * perennial,
        "earthlike seasonal ice {seasonal} perennial {perennial}"
    );

    // Also guards the rain floor: while each month restarted its moisture at
    // half saturation, a water-poor planet still drew about 400 mm on land
    // and this world stayed near 0.45 desert.
    let desert_share = desert.land_moisture().desert_fraction;
    let earth_desert = earth.land_moisture().desert_fraction;
    assert!(
        desert_share > 0.55 && desert_share > earth_desert,
        "desert fraction {desert_share} earthlike {earth_desert}"
    );

    for (sample, target) in [(&water, 0.02), (&archipelago, 0.10)] {
        let got = sample.stats.achieved_land_fraction;
        assert!(
            (got - target).abs() <= 0.01,
            "land fraction {got} target {target}"
        );
    }

    let pangea_share = largest_share(&pangea);
    let earth_share = largest_share(&earth);
    assert!(
        pangea_share > earth_share,
        "pangea largest share {pangea_share} earthlike {earth_share}"
    );
    let shattered_count = landmass_areas(&shattered).len();
    let earth_count = landmass_areas(&earth).len();
    assert!(
        shattered_count > earth_count,
        "shattered landmasses {shattered_count} earthlike {earth_count}"
    );
}
