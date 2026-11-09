use crate::climate::{
    daily_insolation, declination, diffuse_meridional, distance_factor, spin_up_on_land,
};
use crate::grid::CubeSphere;
use crate::{
    generate, NoProgress, PlanetSettings, World, WorldSettings, NH_SUMMER_SOLSTICE,
    NH_WINTER_SOLSTICE,
};

const RADIUS: f64 = 6_371_000.0;

fn earth_orbit() -> PlanetSettings {
    PlanetSettings {
        eccentricity: 0.0,
        ..PlanetSettings::default()
    }
}

fn month_insolation(planet: &PlanetSettings, month: usize, lat: f64) -> f64 {
    let lambda = std::f64::consts::TAU * (month as f64) / 12.0;
    let delta = declination(planet.axial_tilt_deg.to_radians(), lambda);
    let solar = planet.solar_constant()
        * distance_factor(
            planet.eccentricity,
            lambda,
            planet.perihelion_deg.to_radians(),
        );
    daily_insolation(solar, lat, delta)
}

fn annual_at(planet: &PlanetSettings, lat: f64) -> f64 {
    (0..12)
        .map(|month| month_insolation(planet, month, lat))
        .sum::<f64>()
        / 12.0
}

#[test]
fn insolation_matches_earth_limits() {
    let planet = earth_orbit();
    let solar = planet.solar_constant();
    let grid = CubeSphere::new(32, RADIUS).unwrap();
    let mut sum = 0.0;
    let mut area = 0.0;
    for cell in 0..grid.cell_count() {
        let (lat, _) = grid.lat_lon(cell as u32);
        let annual = annual_at(&planet, lat);
        let weight = grid.area_m2(cell as u32);
        sum += annual * weight;
        area += weight;
    }
    let mean = sum / area;
    let target = solar / 4.0;
    let rel = (mean - target).abs() / target;
    assert!(rel < 0.005, "global {mean} target {target} rel {rel}");

    let equator = annual_at(&planet, 0.0);
    assert!(
        (410.0..=425.0).contains(&equator),
        "equator annual {equator}"
    );
    let north = annual_at(&planet, std::f64::consts::FRAC_PI_2);
    let south = annual_at(&planet, -std::f64::consts::FRAC_PI_2);
    assert!(
        (165.0..=180.0).contains(&north),
        "north pole annual {north}"
    );
    assert!(
        (165.0..=180.0).contains(&south),
        "south pole annual {south}"
    );
}

#[test]
fn zero_tilt_has_no_equator_season_and_dark_poles() {
    let planet = PlanetSettings {
        eccentricity: 0.0,
        axial_tilt_deg: 0.0,
        ..PlanetSettings::default()
    };
    let equator: Vec<f64> = (0..12)
        .map(|month| month_insolation(&planet, month, 0.0))
        .collect();
    let spread = equator.iter().copied().fold(f64::MIN, f64::max)
        - equator.iter().copied().fold(f64::MAX, f64::min);
    assert!(spread < 1e-6, "equator spread {spread}");
    assert!(annual_at(&planet, std::f64::consts::FRAC_PI_2).abs() < 1e-8);
    assert!(annual_at(&planet, -std::f64::consts::FRAC_PI_2).abs() < 1e-8);
}

#[test]
fn polar_hour_angle_is_finite() {
    let declinations = [
        0.0,
        0.4,
        -0.4,
        1.0,
        -1.0,
        std::f64::consts::FRAC_PI_2,
        -std::f64::consts::FRAC_PI_2,
    ];
    let latitudes = [
        0.0,
        1.2,
        -1.2,
        std::f64::consts::FRAC_PI_2,
        -std::f64::consts::FRAC_PI_2,
        std::f64::consts::FRAC_PI_2 - 1e-12,
        -std::f64::consts::FRAC_PI_2 + 1e-12,
        std::f64::consts::FRAC_PI_2 - 1e-7,
        -(std::f64::consts::FRAC_PI_2 - 1e-7),
    ];
    for lat in latitudes {
        for decl in declinations {
            let q = daily_insolation(1361.0, lat, decl);
            assert!(q.is_finite() && q >= 0.0, "lat {lat} decl {decl} -> {q}");
        }
    }
}

#[test]
fn band_diffusion_conserves_heat() {
    let mut capacity = vec![0.0; 64];
    let mut temperature = vec![0.0; 64];
    for (band, (c, t)) in capacity.iter_mut().zip(temperature.iter_mut()).enumerate() {
        *c = 1.0e7 + band as f64 * 3.2e6;
        *t = (band as f64 - 31.5) * 0.8;
    }
    let next = diffuse_meridional(&capacity, &temperature, 0.45, 2.6298e6);
    assert!(next.iter().all(|value| value.is_finite()));
    let before: f64 = capacity.iter().zip(&temperature).map(|(c, t)| c * t).sum();
    let after: f64 = capacity.iter().zip(&next).map(|(c, t)| c * t).sum();
    let rel = (after - before).abs() / before.abs();
    assert!(
        rel < 1e-9,
        "relative heat drift {rel} before {before} after {after}"
    );
}

fn coarse_range(world: &World) -> String {
    let t = &world.climate_field.t_c;
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    for value in t {
        lo = lo.min(*value);
        hi = hi.max(*value);
    }
    format!("{} min {lo} max {hi}", coarse_annual_t(world))
}

fn coarse_annual_t(world: &World) -> String {
    let grid = &world.climate_field.grid;
    let n = grid.cell_count();
    let mut bands = [(0.0, 0.0); 6];
    let mut global = (0.0, 0.0);
    for cell in 0..n {
        let lat = grid.lat_lon(cell as u32).0.to_degrees();
        let mut annual = 0.0;
        for month in 0..12 {
            annual += f64::from(world.climate_field.t_c[month * n + cell]);
        }
        annual /= 12.0;
        let area = grid.area_m2(cell as u32);
        global.0 += annual * area;
        global.1 += area;
        let idx = if lat < -70.0 {
            0
        } else if lat < -30.0 {
            1
        } else if lat < 0.0 {
            2
        } else if lat < 30.0 {
            3
        } else if lat < 70.0 {
            4
        } else {
            5
        };
        bands[idx].0 += annual * area;
        bands[idx].1 += area;
    }
    format!(
        "coarse {:.2} bands {:.1}/{:.1}/{:.1}/{:.1}/{:.1}/{:.1}",
        global.0 / global.1,
        bands[0].0 / bands[0].1.max(1.0),
        bands[1].0 / bands[1].1.max(1.0),
        bands[2].0 / bands[2].1.max(1.0),
        bands[3].0 / bands[3].1.max(1.0),
        bands[4].0 / bands[4].1.max(1.0),
        bands[5].0 / bands[5].1.max(1.0)
    )
}

fn world_at(offset: f64) -> World {
    world_seed_offset(1, offset)
}

fn world_seed(seed: u64) -> World {
    world_seed_offset(seed, 0.0)
}

fn world_seed_offset(seed: u64, offset: f64) -> World {
    generate(
        &WorldSettings {
            resolution: 64,
            seed,
            planet: PlanetSettings {
                temperature_offset_c: offset,
                ..PlanetSettings::default()
            },
            ..WorldSettings::default()
        },
        &mut NoProgress,
    )
    .unwrap()
}

fn spin_note(world: &World) -> String {
    let field = &world.climate_field;
    if field.spinup_pair_t.is_empty() {
        return format!(
            "residual {} years {} converged {}",
            field.spinup_residual_k, field.spinup_years, field.converged
        );
    }
    let n = field.grid.cell_count();
    let pair = &field.spinup_pair_t;
    let mut worst = 0.0f32;
    let mut note = format!("residual {}", field.spinup_residual_k);
    for month in 0..12 {
        for cell in 0..n {
            let idx = month * n + cell;
            let delta = (pair[12 * n + idx] - pair[idx]).abs();
            if delta >= worst {
                worst = delta;
                let lat = field.grid.lat_lon(cell as u32).0.to_degrees();
                note = format!(
                    "residual {} worst m {month} lat {lat:.1} d {delta:.2} prev {} last {} ocean {}",
                    field.spinup_residual_k,
                    pair[idx],
                    pair[12 * n + idx],
                    world.ocean[cell]
                );
            }
        }
    }
    note
}

fn maritime_winter(world: &World) -> (f64, f64, usize, usize) {
    let grid = &world.climate_field.grid;
    let n = grid.cell_count();
    let winter = NH_WINTER_SOLSTICE * n;
    let mut coast = 0.0;
    let mut coast_a = 0.0;
    let mut coast_n = 0usize;
    let mut inland = 0.0;
    let mut inland_a = 0.0;
    let mut inland_n = 0usize;
    for cell in 0..n {
        if world.ocean[cell] != 0 {
            continue;
        }
        let lat = grid.lat_lon(cell as u32).0.to_degrees();
        if !(40.0..=60.0).contains(&lat) {
            continue;
        }
        let maritime = f64::from(world.climate_field.maritime[cell]);
        let temperature = f64::from(world.climate_field.t_c[winter + cell]);
        let area = grid.area_m2(cell as u32);
        if maritime > 0.5 {
            coast += temperature * area;
            coast_a += area;
            coast_n += 1;
        } else if maritime < 0.1 {
            inland += temperature * area;
            inland_a += area;
            inland_n += 1;
        }
    }
    (
        mean(coast, coast_a),
        mean(inland, inland_a),
        coast_n,
        inland_n,
    )
}

#[test]
fn earthlike_n64_climate_is_in_range() {
    assert_eq!(NH_SUMMER_SOLSTICE, 3);
    assert_eq!(NH_WINTER_SOLSTICE, 9);
    let world = world_at(0.0);
    let coast = crate::climate::westerly_coast_contrast(
        &world.climate_field.grid,
        &world.ocean,
        &world.climate_field.t_c,
        &world.climate_field.wind,
    );
    assert!(
        coast.coast_c.is_finite()
            && coast.inland_c.is_finite()
            && coast.coast_n > 0
            && coast.inland_n > 0,
        "coast {} inland {} n {}/{}",
        coast.coast_c,
        coast.inland_c,
        coast.coast_n,
        coast.inland_n
    );
    let stats = world.climate.stats;
    assert!(
        (8.0..=20.0).contains(&stats.t_global_c),
        "global T {} equator {} polar {} P {} years {} residual {} converged {} sea-level {}",
        stats.t_global_c,
        stats.t_equator_c,
        stats.t_polar_c,
        stats.p_global_mm,
        stats.spinup_years,
        world.climate_field.spinup_residual_k,
        world.climate_field.converged,
        coarse_range(&world)
    );
    assert!(stats.t_equator_c > 20.0, "equator T {}", stats.t_equator_c);
    assert!(stats.t_polar_c < -5.0, "polar T {}", stats.t_polar_c);
    assert!(
        (600.0..=1400.0).contains(&stats.p_global_mm),
        "global P {} T {} equator T {} polar T {} eqP {} subP {} years {} delta {}",
        stats.p_global_mm,
        stats.t_global_c,
        stats.t_equator_c,
        stats.t_polar_c,
        stats.p_equator_mm,
        stats.p_subtropical_land_mm,
        stats.spinup_years,
        world.climate_field.spinup_residual_k
    );
    assert!(
        stats.p_equator_mm > stats.p_subtropical_land_mm,
        "equator P {} subtropical land P {}",
        stats.p_equator_mm,
        stats.p_subtropical_land_mm
    );
    assert_spinup(&world);
    assert!(stats.spinup_years <= 40, "years {}", stats.spinup_years);
    assert!(
        (stats.phi_h_deg - 30.0).abs() < 1e-9,
        "phi_h {}",
        stats.phi_h_deg
    );
    let (land_summer, land_winter, ocean_summer, ocean_winter, land_n, ocean_n) =
        continentality(&world);
    assert!(
        land_n > 0 && ocean_n > 0,
        "land cells {land_n} ocean cells {ocean_n}"
    );
    assert!(
        land_summer > ocean_summer,
        "NH summer land {land_summer} ocean {ocean_summer} n={land_n}/{ocean_n}"
    );
    assert!(
        land_winter < ocean_winter,
        "NH winter land {land_winter} ocean {ocean_winter} n={land_n}/{ocean_n}"
    );
    assert!(world.climate.t_annual_c.iter().all(|v| v.is_finite()));
    assert!(world.climate.p_annual_mm.iter().all(|v| v.is_finite()));
    assert!(world.climate_field.wind.iter().all(|v| v.is_finite()));
    // Sea-ice fronts may still creep at year 40; the planet must have settled.
    let drift = spin_drift_rms(&world);
    assert!(
        world.climate_field.converged || drift < 0.1,
        "seed 1 drift {drift} {}",
        spin_note(&world)
    );
    let (maritime, interior, maritime_n, interior_n) = maritime_winter(&world);
    assert!(
        maritime_n > 0 && interior_n > 0,
        "maritime cells {maritime_n} interior cells {interior_n}"
    );
    assert!(
        maritime > interior,
        "NH winter maritime {maritime} interior {interior} n={maritime_n}/{interior_n}"
    );
}

fn assert_spinup(world: &World) {
    let field = &world.climate_field;
    let residual = field.spinup_residual_k;
    // The worst cell sits on a creeping sea-ice front; `spin_drift_rms` checks
    // the planet as a whole.
    assert!(
        field.converged || residual < 3.0,
        "residual {residual} converged {}",
        field.converged
    );
    if field.converged {
        assert!(residual < 0.02, "converged residual {residual}");
        assert!(field.spinup_pair_t.is_empty());
        return;
    }
    assert_eq!(field.spinup_years, 40, "unconverged years");
    assert_pair_mean(&field.t_c, &field.spinup_pair_t, "temperature");
}

fn assert_pair_mean(stored: &[f32], pair: &[f32], label: &str) {
    assert_eq!(pair.len(), stored.len() * 2, "{label} pair");
    let half = stored.len();
    let mut mean = Vec::with_capacity(half);
    for i in 0..half {
        mean.push(0.5 * (pair[i] + pair[half + i]));
    }
    assert_close(stored, &mean, label);
}

fn assert_close(stored: &[f32], mean: &[f32], label: &str) {
    assert_eq!(stored.len(), mean.len(), "{label}");
    for (i, (got, expect)) in stored.iter().zip(mean).enumerate() {
        let err = (got - expect).abs();
        assert!(err < 1e-3, "{label} {i} stored {got} mean {expect}");
    }
}

fn continentality(world: &World) -> (f64, f64, f64, f64, usize, usize) {
    let grid = &world.climate_field.grid;
    let n = grid.cell_count();
    let summer = NH_SUMMER_SOLSTICE * n;
    let winter = NH_WINTER_SOLSTICE * n;
    let mut land_s = 0.0;
    let mut land_w = 0.0;
    let mut land_a = 0.0;
    let mut ocean_s = 0.0;
    let mut ocean_w = 0.0;
    let mut ocean_a = 0.0;
    let mut land_n = 0usize;
    let mut ocean_n = 0usize;
    for cell in 0..n {
        let lat = grid.lat_lon(cell as u32).0.to_degrees();
        if !(30.0..=55.0).contains(&lat) {
            continue;
        }
        let area = grid.area_m2(cell as u32);
        let warm = f64::from(world.climate_field.t_c[summer + cell]);
        let cold = f64::from(world.climate_field.t_c[winter + cell]);
        if world.ocean[cell] == 0 && interior_land(world, cell as u32) {
            land_s += warm * area;
            land_w += cold * area;
            land_a += area;
            land_n += 1;
        } else if world.ocean[cell] == 1 {
            ocean_s += warm * area;
            ocean_w += cold * area;
            ocean_a += area;
            ocean_n += 1;
        }
    }
    (
        mean(land_s, land_a),
        mean(land_w, land_a),
        mean(ocean_s, ocean_a),
        mean(ocean_w, ocean_a),
        land_n,
        ocean_n,
    )
}

fn interior_land(world: &World, cell: u32) -> bool {
    let neighbors = world.grid.neighbors(cell);
    let mut seen = 0;
    for slot in [0usize, 2, 4, 6] {
        let nb = neighbors[slot];
        if nb == crate::grid::NONE {
            return false;
        }
        if world.ocean[nb as usize] != 0 {
            return false;
        }
        seen += 1;
    }
    seen == 4
}

fn mean(sum: f64, area: f64) -> f64 {
    if area > 0.0 {
        sum / area
    } else {
        0.0
    }
}

#[test]
fn temperature_offset_is_monotonic() {
    let cold = world_at(-15.0);
    let base = world_at(0.0);
    let hot = world_at(10.0);
    assert!(
        cold.climate.stats.t_global_c < base.climate.stats.t_global_c,
        "cold {} converged {} residual {} base {} converged {} residual {}",
        cold.climate.stats.t_global_c,
        cold.climate_field.converged,
        cold.climate_field.spinup_residual_k,
        base.climate.stats.t_global_c,
        base.climate_field.converged,
        base.climate_field.spinup_residual_k
    );
    let cold_ice = ice_months(&cold);
    let base_ice = ice_months(&base);
    let hot_ice = ice_months(&hot);
    assert!(
        cold_ice > base_ice && hot_ice < base_ice,
        "ice cold {cold_ice} base {base_ice} hot {hot_ice}"
    );
}

fn ice_months(world: &World) -> f64 {
    let mut sum = 0.0;
    let mut area = 0.0;
    for cell in 0..world.grid.cell_count() {
        let weight = world.grid.area_m2(cell as u32);
        sum += f64::from(world.climate.sea_ice_months[cell]) * weight;
        area += weight;
    }
    sum / area
}

#[test]
fn climate_matches_across_thread_counts() {
    let one = generate_threads(1);
    let four = generate_threads(4);
    assert_eq!(one.elevation_hash(), four.elevation_hash());
    assert_eq!(one.climate_hash(), four.climate_hash());
    assert_eq!(one.climate.t_annual_c, four.climate.t_annual_c);
    assert_eq!(one.climate.p_annual_mm, four.climate.p_annual_mm);
    assert_eq!(one.climate.sea_ice_months, four.climate.sea_ice_months);
    assert_eq!(one.climate_field.t_c, four.climate_field.t_c);
    assert_eq!(one.climate_field.p_mm, four.climate_field.p_mm);
    assert_eq!(one.climate_field.wind, four.climate_field.wind);
    assert_eq!(one.climate_field.q_wm2, four.climate_field.q_wm2);
    assert_eq!(one.climate_field.s_land, four.climate_field.s_land);
    assert_eq!(one.climate_field.s_ocean, four.climate_field.s_ocean);
    assert_eq!(one.climate_field.maritime, four.climate_field.maritime);
}

fn generate_threads(threads: usize) -> World {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    pool.install(|| world_at(0.0))
}

#[test]
fn nonconverged_spin_stores_the_two_year_mean() {
    let current_t = [1.0, 3.0];
    let previous_t = [5.0, 7.0];
    let published = crate::climate::publish_spin(&current_t, &previous_t, false);
    assert!((published.temperature[0] - 3.0).abs() < 1e-12);
    assert!((published.temperature[1] - 5.0).abs() < 1e-12);
    let kept = crate::climate::publish_spin(&current_t, &previous_t, true);
    assert_eq!(kept.temperature, current_t);
}

/// Area-weighted RMS same-month change over the last raw spin-up year, K.
/// Zero after a converged spin-up.
fn spin_drift_rms(world: &World) -> f64 {
    let field = &world.climate_field;
    let pair = &field.spinup_pair_t;
    if pair.is_empty() {
        return 0.0;
    }
    let n = field.grid.cell_count();
    let half = pair.len() / 2;
    let mut sum = 0.0;
    let mut area = 0.0;
    for index in 0..half {
        let cell_area = field.grid.area_m2((index % n) as u32);
        let change = f64::from(pair[half + index] - pair[index]);
        sum += cell_area * change * change;
        area += cell_area;
    }
    (sum / area).sqrt()
}

/// Sea-ice fronts settle slowly: cover follows the regional temperature, so a
/// front moves until latitude pins it, and 40 years can leave about 1 K at the
/// edge cells. The planet as a whole must have settled.
#[test]
fn default_earth_spinup_settles_for_seed_7() {
    let world = world_seed(7);
    let drift = spin_drift_rms(&world);
    assert!(
        world.climate_field.converged || drift < 0.1,
        "seed 7 drift {drift} {}",
        spin_note(&world)
    );
}

/// Meridians crossing the ocean from 40° to the pole must pass the sea-ice
/// edge once. When cover followed each cell's own temperature, every cell was
/// bistable and the edge came out as alternating frozen and open rows.
#[test]
fn sea_ice_edges_are_single_fronts() {
    for seed in [1, 7] {
        let world = world_seed(seed);
        let field = &world.climate_field;
        let n = field.grid.cell_count();
        let mut extra = 0;
        for step in 0..72 {
            let lon = -180.0 + 5.0 * f64::from(step);
            for sign in [1.0, -1.0] {
                let mut previous: Option<bool> = None;
                let mut toggles = 0;
                for k in 0..180 {
                    let lat = sign * (40.0 + 0.25 * f64::from(k));
                    let point = crate::direction_from_lat_lon(lat, lon);
                    if world.ocean[world.grid.cell_at(point) as usize] != 1 {
                        previous = None;
                        continue;
                    }
                    let cell = field.grid.cell_at(point) as usize;
                    let cover: f32 = (0..12).map(|m| field.s_ocean[m * n + cell]).sum();
                    let icy = cover / 12.0 > 0.5;
                    if previous.is_some_and(|old| old != icy) {
                        toggles += 1;
                    }
                    previous = Some(icy);
                }
                extra += toggles.max(1) - 1;
            }
        }
        // Per-cell bistability gave 80–95 extra crossings per seed; a polynya
        // by a coast can add one or two.
        assert!(
            extra <= 4,
            "seed {seed}: {extra} extra sea-ice edge crossings"
        );
    }
}

#[test]
fn continental_rain_reaches_mid_latitudes() {
    // Seed 7 is the most arid world checked: once moisture carries across
    // months (no start-up floor), it is about 0.38 ice-free desert at n = 64
    // and n = 512, against 0.08–0.21 for eight other seeds at n = 256, with
    // about 520 mm of land rain against a median near 780 mm. So this test allows
    // 0.40 desert and 500 mm. The other rain bounds are the full targets.
    let world = world_seed(7);
    let rain = world.land_moisture();
    assert!(
        rain.desert_fraction <= 0.40,
        "ice-free desert {}",
        rain.desert_fraction
    );
    assert!(
        rain.p_midlat_mm >= 350.0,
        "mid-latitude land P {}",
        rain.p_midlat_mm
    );
    assert!(
        (850.0..=1200.0).contains(&world.climate.stats.p_global_mm),
        "global P {}",
        world.climate.stats.p_global_mm
    );
    assert!(
        (500.0..=1000.0).contains(&rain.p_land_mm),
        "land P {}",
        rain.p_land_mm
    );
}

#[test]
fn derived_earth_orbit() {
    let planet = PlanetSettings::default();
    assert!((planet.solar_constant() - 1361.0).abs() < 1e-9);
    assert!((planet.year_seconds() - 3.15576e7).abs() < 1e-6);
    assert!((planet.month_seconds() * 12.0 - planet.year_seconds()).abs() < 1e-6);
}

/// Climate cells whose annual sea-ice state disagrees with every neighbour.
fn isolated_ice_flips(world: &World) -> (usize, usize) {
    let field = &world.climate_field;
    let grid = &field.grid;
    let n = grid.cell_count();
    let icy: Vec<bool> = (0..n)
        .map(|cell| {
            let total: f32 = (0..12).map(|m| field.s_ocean[m * n + cell]).sum();
            total / 12.0 > 0.5
        })
        .collect();
    let mut flips = 0;
    for cell in 0..n {
        let neighbours: Vec<u32> = grid
            .neighbors(cell as u32)
            .into_iter()
            .filter(|nb| *nb != crate::grid::NONE)
            .collect();
        if neighbours.iter().all(|nb| icy[*nb as usize] != icy[cell]) {
            flips += 1;
        }
    }
    (flips, n)
}

/// Cold, bistable planets must not leave single climate cells on the other
/// ice-albedo branch (open water inside a frozen equator, or the reverse).
#[test]
fn cold_worlds_have_no_single_cell_ice_flips() {
    for offset in [-15.0, -25.0] {
        let world = generate(
            &WorldSettings {
                resolution: 128,
                seed: 7,
                planet: PlanetSettings {
                    temperature_offset_c: offset,
                    ..PlanetSettings::default()
                },
                ..WorldSettings::default()
            },
            &mut NoProgress,
        )
        .unwrap();
        let (flips, cells) = isolated_ice_flips(&world);
        assert!(
            flips * 1000 <= cells,
            "offset {offset}: {flips} isolated ice flips among {cells} climate cells"
        );
    }
}

/// A polar band that is mostly land with a few ocean cells. Meridional
/// transport used to raise every cell of a band by the same temperature, so
/// those ocean cells took the band's whole heat convergence and ran past
/// 200 °C. Transport is now a flux per unit area.
#[test]
fn ocean_cells_in_a_land_band_stay_physical() {
    let grid = CubeSphere::new(32, RADIUS).unwrap();
    let land: Vec<f64> = (0..grid.cell_count())
        .map(|cell| {
            let lat = grid.lat_lon(cell as u32).0.to_degrees();
            if lat < -62.0 && cell % 9 != 0 {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    let ocean_in_cap = (0..grid.cell_count())
        .filter(|cell| grid.lat_lon(*cell as u32).0.to_degrees() < -62.0 && land[*cell] == 0.0)
        .count();
    assert!(ocean_in_cap > 0, "no ocean cell inside the polar land cap");
    let t = spin_up_on_land(&grid, &land, &PlanetSettings::default());
    let n = grid.cell_count();
    for (index, value) in t.iter().enumerate() {
        let cell = index % n;
        let lat = grid.lat_lon(cell as u32).0.to_degrees();
        assert!(
            value.is_finite() && (-80.0..=50.0).contains(value),
            "month {} cell {cell} lat {lat:.1} land {} T {value}",
            index / n,
            land[cell]
        );
    }
}
