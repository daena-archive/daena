use std::env;
use std::time::Instant;

use daena_physical::engine::{self, Settings, WorldPreset};
use daena_physical::history::{
    derive_historical_products, solve_epoch, HistoricalForcingParameters,
};
use daena_physical::planetary::PlanetaryConfiguration;
use daena_physical::{NoopProgress, DEFAULT_RADIUS_METRES};

fn option<T: std::str::FromStr>(args: &[String], name: &str) -> Result<Option<T>, String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| {
            window[1]
                .parse::<T>()
                .map_err(|_| format!("{name} has an invalid value: {}", window[1]))
        })
        .transpose()
}

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let resolution = option(&args, "--resolution")?.unwrap_or(512);
    let seed = option(&args, "--seed")?.unwrap_or(831_429);
    let preset_name = args
        .windows(2)
        .find(|window| window[0] == "--preset")
        .map(|window| window[1].clone())
        .unwrap_or_else(|| "earthlike".into());
    let preset =
        WorldPreset::parse(&preset_name).ok_or_else(|| format!("unknown preset {preset_name}"))?;
    let warmup: u32 = option(&args, "--warmup")?.unwrap_or(1);
    let repeat: u32 = option(&args, "--repeat")?.unwrap_or(1);
    let offset_years: i64 = option(&args, "--offset-years")?.unwrap_or(-8_000);
    println!(
        "pid={} resolution={} seed={} preset={} offsetYears={}",
        std::process::id(),
        resolution,
        seed,
        preset.as_str(),
        offset_years
    );
    let settings = Settings::from_parts(
        seed,
        preset,
        resolution,
        DEFAULT_RADIUS_METRES,
        0,
        PlanetaryConfiguration::earth_like(),
    );
    let generate_started = Instant::now();
    let products = engine::generate(settings, &mut NoopProgress).map_err(|e| e.to_string())?;
    let generation_ms = generate_started.elapsed().as_secs_f64() * 1000.0;
    let source = products.source().to_vec();
    let decoded = engine::decode_source(&source).map_err(|e| e.to_string())?;
    let forcing = HistoricalForcingParameters::default_for_seed(decoded.seed());
    let planetary = PlanetaryConfiguration::earth_like();
    let present = daena_physical::derived_cache::StaticDerivedPhysics::new(
        products.view().clone(),
        products.climate().clone(),
        products.hydrology().clone(),
        products.hazards().clone(),
        products.derived_geojson().to_string(),
        products.hashes(),
    );
    let derive = || -> Result<_, String> {
        let solved = solve_epoch(&decoded, forcing, offset_years, &NoopProgress)
            .map_err(|e| e.to_string())?;
        derive_historical_products(
            &decoded,
            &present,
            forcing,
            offset_years,
            planetary,
            &mut NoopProgress,
        )
        .map(|historical| (solved, historical))
        .map_err(|e| e.to_string())
    };
    for _ in 0..warmup {
        derive()?;
    }
    let mut times_ms = Vec::with_capacity(repeat as usize);
    let mut last = None;
    for _ in 0..repeat {
        let started = Instant::now();
        let historical = derive()?;
        times_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        last = Some(historical);
    }
    let mean = if times_ms.is_empty() {
        0.0
    } else {
        times_ms.iter().sum::<f64>() / times_ms.len() as f64
    };
    let (solved, historical) = last.ok_or_else(|| "no epoch runs".to_string())?;
    println!(
        "{{\"resolution\":{},\"seed\":{},\"preset\":\"{}\",\"offsetYears\":{},\"warmup\":{},\"repeat\":{},\"generationMs\":{:.3},\"meanMs\":{:.3},\"seaLevelMm\":{},\"temperatureOffsetCentiC\":{},\"laggedTemperatureOffsetCentiC\":{},\"meanTemperatureCentiC\":{},\"meanPrecipitationMm\":{},\"landIceM3\":{},\"balanceErrorM3\":{},\"runsMs\":[{}]}}",
        resolution,
        seed,
        preset.as_str(),
        offset_years,
        warmup,
        repeat,
        generation_ms,
        mean,
        solved.sea_level_mm,
        solved.temperature_offset_centi_c,
        solved.lagged_temperature_offset_centi_c,
        historical.products.climate.metrics.mean_temperature_centi_c,
        historical.products.climate.metrics.mean_precipitation_mm_per_year,
        historical.metrics.land_ice_m3,
        historical.metrics.balance_error_m3,
        times_ms
            .iter()
            .map(|value| format!("{value:.3}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(())
}
