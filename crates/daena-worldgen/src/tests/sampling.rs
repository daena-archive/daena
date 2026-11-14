use crate::{generate, resample_equirect_f32, NoProgress, Preset, WorldSettings};

fn settings(resolution: u32) -> WorldSettings {
    WorldSettings {
        resolution,
        ..Preset::Earthlike.settings()
    }
}

#[test]
fn constant_fields_stay_constant_at_poles_and_face_seams() {
    let settings = settings(32);
    let world = generate(&settings, &mut NoProgress).unwrap();
    let n = world.grid.cell_count();
    let constant = vec![123.456f32; n];
    let view = resample_equirect_f32(&world.grid, &constant, 64, 32);
    assert!(view.iter().all(|value| *value == 123.456));
    for (lat, lon) in [
        (0.0, 0.0),
        (90.0, 0.0),
        (-90.0, 180.0),
        (0.0, 45.0),
        (35.264_389_682_754_654, 45.0),
        (35.264_389_682_754_654, 135.0),
        (35.264_389_682_754_654, -45.0),
        (35.264_389_682_754_654, -135.0),
        (-35.264_389_682_754_654, 45.0),
        (-35.264_389_682_754_654, 135.0),
        (-35.264_389_682_754_654, -45.0),
        (-35.264_389_682_754_654, -135.0),
        (-33.0, -179.0),
        (10.0, 179.0),
    ] {
        let sample = world
            .grid
            .sample_bilinear(&constant, crate::direction_from_lat_lon(lat, lon));
        assert_eq!(sample, 123.456);
    }
}

#[test]
fn point_sample_at_cell_centre_returns_that_cell() {
    let settings = settings(32);
    let mut world = generate(&settings, &mut NoProgress).unwrap();
    world.elevation_m = vec![321.0; world.grid.cell_count()];
    for cell in [0u32, 17, 255, 777] {
        let (lat, lon) = world.grid.lat_lon(cell);
        let sample = world.sample_point(lat.to_degrees(), lon.to_degrees());
        assert_eq!(sample.cell, cell);
        assert!((sample.elevation_m - 321.0).abs() < 1e-3);
    }
}

#[test]
fn monthly_point_sample_matches_the_fine_month_field() {
    let settings = settings(128);
    let world = generate(&settings, &mut NoProgress).unwrap();
    let month = crate::climate::NH_SUMMER_SOLSTICE;
    let field: Vec<f32> = (0..world.grid.cell_count())
        .map(|cell| world.climate_month_at(cell as u32, month).0)
        .collect();
    for (lat, lon) in [
        (0.0, 0.0),
        (0.0, 90.0),
        (0.0, 180.0),
        (0.0, 45.0),
        (35.264_389_682_754_654, 45.0),
        (35.264_389_682_754_654, 135.0),
        (35.264_389_682_754_654, -45.0),
        (35.264_389_682_754_654, -135.0),
        (-35.264_389_682_754_654, 45.0),
        (-35.264_389_682_754_654, 135.0),
        (-35.264_389_682_754_654, -45.0),
        (-35.264_389_682_754_654, -135.0),
        (-45.0, -45.0),
        (89.0, 0.0),
        (-89.0, 0.0),
        (90.0, 0.0),
        (-90.0, 180.0),
    ] {
        let expected = world
            .grid
            .sample_bilinear(&field, crate::direction_from_lat_lon(lat, lon));
        let actual = world.climate_month_at_point(lat, lon, month).0;
        assert_eq!(actual.to_bits(), expected.to_bits(), "at {lat},{lon}");
    }
    let view = world.equirect_view(64, 32).unwrap();
    for row in 0..32u32 {
        let lat = -90.0 + 180.0 * (f64::from(row) + 0.5) / 32.0;
        for col in 0..64u32 {
            let lon = -180.0 + 360.0 * (f64::from(col) + 0.5) / 64.0;
            let index = (row * 64 + col) as usize;
            let (summer, _) = world.climate_month_at_point(lat, lon, month);
            assert_eq!(view.t_nh_summer_c[index].to_bits(), summer.to_bits());
            let (winter, _) =
                world.climate_month_at_point(lat, lon, crate::climate::NH_WINTER_SOLSTICE);
            assert_eq!(view.t_nh_winter_c[index].to_bits(), winter.to_bits());
        }
    }
}

#[test]
fn elevation_only_view_matches_full_view_and_cancels() {
    let settings = settings(32);
    let world = generate(&settings, &mut NoProgress).unwrap();
    let full = world.equirect_view(64, 32).unwrap();
    let only = world
        .equirect_elevation_with_cancel(64, 32, &|| false)
        .unwrap();
    assert_eq!(only.len(), full.elevation_m.len());
    for (index, value) in only.iter().enumerate() {
        assert_eq!(
            value.to_bits(),
            full.elevation_m[index].to_bits(),
            "at {index}"
        );
    }
    assert_eq!(
        world
            .equirect_elevation_with_cancel(64, 32, &|| true)
            .unwrap_err(),
        crate::WorldError::Cancelled
    );
}

#[test]
fn equirect_view_reports_memory_at_1024_by_512() {
    let settings = settings(32);
    let world = generate(&settings, &mut NoProgress).unwrap();
    let view = world.equirect_view(1024, 512).unwrap();
    let climate_bytes = [
        &view.climate.t_annual_c,
        &view.climate.t_warm_c,
        &view.climate.t_cold_c,
        &view.climate.p_annual_mm,
        &view.climate.p_dry_month_mm,
        &view.climate.pet_annual_mm,
        &view.climate.runoff_mm,
        &view.climate.ice_balance_mm,
    ]
    .iter()
    .map(|field| field.len() * std::mem::size_of::<f32>())
    .sum::<usize>()
        + view.climate.sea_ice_months.len();
    let bytes = climate_bytes
        + view.elevation_m.len() * std::mem::size_of::<f32>()
        + view.ocean.len()
        + view.t_nh_summer_c.len() * std::mem::size_of::<f32>()
        + view.t_nh_winter_c.len() * std::mem::size_of::<f32>()
        + view.p_nh_summer_mm.len() * std::mem::size_of::<f32>()
        + view.p_nh_winter_mm.len() * std::mem::size_of::<f32>()
        + view.biome.len()
        + view.lake_id.len() * std::mem::size_of::<u32>()
        + view.discharge_m3s.len() * std::mem::size_of::<f32>()
        + view.ice_cover.len()
        + view.hazards.quake_rate_km2.len() * std::mem::size_of::<f32>()
        + view.hazards.quake_hazard.len()
        + view.hazards.volcanic_hazard.len()
        + view.hazards.drought.len()
        + view.hazards.heat_wave.len()
        + view.hazards.extreme_rain.len()
        + view.hazards.flood.len()
        + (view.storms.genesis_km2.len()
            + view.storms.track_km2.len()
            + view.storms.intensity.len()
            + view.storms.genesis_monthly_km2.len())
            * std::mem::size_of::<f32>();
    eprintln!("equirect 1024x512 view bytes: {bytes}");
    let expected = 1024 * 512;
    assert_eq!(view.elevation_m.len(), expected);
    assert_eq!(view.t_nh_summer_c.len(), expected);
    assert_eq!(view.hazards.flood.len(), expected);
}
