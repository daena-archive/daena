use std::env;
use std::time::Instant;

use daena_physical::derived_cache::StaticDerivedPhysics;
use daena_physical::history::{
    derive_historical_world_from_static, derive_historical_world_with_planet,
    HistoricalForcingParameters, HistoricalWorld,
};
use daena_physical::planetary::PlanetaryConfiguration;
use daena_physical::{
    generate_world, GenerationSettings, NoopProgress, DEFAULT_RADIUS_METRES,
    PRODUCTION_DEFAULT_HEIGHT, PRODUCTION_DEFAULT_WIDTH,
};

fn option_u32(args: &[String], name: &str) -> Result<Option<u32>, String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].parse::<u32>())
        .transpose()
        .map_err(|error| format!("{name} must be an unsigned integer: {error}"))
}

fn option_i64(args: &[String], name: &str) -> Result<Option<i64>, String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].parse::<i64>())
        .transpose()
        .map_err(|error| format!("{name} must be an integer: {error}"))
}

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let width = option_u32(&args, "--width")?.unwrap_or(PRODUCTION_DEFAULT_WIDTH);
    let height = option_u32(&args, "--height")?.unwrap_or(PRODUCTION_DEFAULT_HEIGHT);
    let seed = option_u32(&args, "--seed")?.unwrap_or(831_429);
    let warmup = option_u32(&args, "--warmup")?.unwrap_or(1);
    let repeat = option_u32(&args, "--repeat")?.unwrap_or(1);
    let offset_years = option_i64(&args, "--offset-years")?.unwrap_or(0);
    let use_static = args.iter().any(|arg| arg == "--static");
    let settings = GenerationSettings {
        width,
        height,
        radius_metres: DEFAULT_RADIUS_METRES,
        target_land_fraction_ppm: 300_000,
    };
    println!(
        "pid={} width={} height={} seed={} offsetYears={} static={}",
        std::process::id(),
        width,
        height,
        seed,
        offset_years,
        use_static
    );
    let generate_started = Instant::now();
    let world = generate_world(settings, seed, 0, &mut NoopProgress).map_err(|e| e.to_string())?;
    let generation_ms = generate_started.elapsed().as_secs_f64() * 1000.0;
    let inventory = world.report.reference_water_inventory_m3;
    let forcing =
        HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index);
    let physics = StaticDerivedPhysics::from_world(&world).map_err(|e| e.to_string())?;
    let planetary = PlanetaryConfiguration::earth_like();
    let derive = |progress: &mut NoopProgress| -> Result<HistoricalWorld, String> {
        if use_static {
            derive_historical_world_from_static(
                &world.field,
                &physics,
                inventory,
                Some(&world.tectonics.crust_by_cell),
                forcing,
                offset_years,
                progress,
            )
            .map_err(|error| error.to_string())
        } else {
            derive_historical_world_with_planet(
                &world.field,
                inventory,
                Some(&world.tectonics.crust_by_cell),
                forcing,
                offset_years,
                planetary,
                progress,
            )
            .map_err(|error| error.to_string())
        }
    };
    for _ in 0..warmup {
        derive(&mut NoopProgress)?;
    }
    let mut times_ms = Vec::with_capacity(repeat as usize);
    let mut last: Option<HistoricalWorld> = None;
    for _ in 0..repeat {
        let started = Instant::now();
        let historical = derive(&mut NoopProgress)?;
        times_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        last = Some(historical);
    }
    let mean = if times_ms.is_empty() {
        0.0
    } else {
        times_ms.iter().sum::<f64>() / times_ms.len() as f64
    };
    let historical = last.ok_or_else(|| "no epoch runs".to_string())?;
    println!(
        "{{\"width\":{},\"height\":{},\"seed\":{},\"offsetYears\":{},\"static\":{},\"warmup\":{},\"repeat\":{},\"generationMs\":{:.3},\"meanMs\":{:.3},\"seaLevelMm\":{},\"temperatureOffsetCentiC\":{},\"laggedTemperatureOffsetCentiC\":{},\"meanTemperatureCentiC\":{},\"meanPrecipitationMm\":{},\"landIceM3\":{},\"balanceErrorM3\":{},\"transportIterations\":{},\"runsMs\":[{}]}}",
        width,
        height,
        seed,
        offset_years,
        use_static,
        warmup,
        repeat,
        generation_ms,
        mean,
        historical.metrics.sea_level_mm,
        historical.metrics.temperature_offset_centi_c,
        historical.metrics.lagged_temperature_offset_centi_c,
        historical.climate.metrics.mean_temperature_centi_c,
        historical.climate.metrics.mean_precipitation_mm_per_year,
        historical.metrics.land_ice_m3,
        historical.metrics.balance_error_m3,
        historical.climate.metrics.transport_iterations,
        times_ms
            .iter()
            .map(|value| format!("{value:.3}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(())
}
