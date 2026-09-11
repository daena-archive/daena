use std::env;
use std::thread;
use std::time::{Duration, Instant};

use daena_physical::climate::{
    derive_current_climate, ClimateField, ClimateSettings, BIOME_CLASS_MAX,
};
use daena_physical::{
    Grid, NoopProgress, PhysicalField, DEFAULT_RADIUS_METRES, PRODUCTION_DEFAULT_HEIGHT,
    PRODUCTION_DEFAULT_WIDTH,
};

fn option_u32(args: &[String], name: &str) -> Result<Option<u32>, String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].parse::<u32>())
        .transpose()
        .map_err(|error| format!("{name} must be an unsigned integer: {error}"))
}

fn synthetic_field(grid: Grid, seed: u32) -> PhysicalField {
    let mut elevations = vec![-4_000_000; grid.sample_count()];
    let equator = grid.height / 2;
    let row0 = equator.saturating_sub(grid.height / 8);
    let row1 = (equator + grid.height / 8 + 1).min(grid.height);
    let col0 = grid.width / 5;
    let col1 = (grid.width * 3) / 5;
    for row in row0..row1 {
        for col in col0..col1 {
            elevations[grid.index(row, col)] = 80_000;
        }
    }
    PhysicalField {
        grid,
        seed,
        retry_index: 0,
        target_land_fraction_ppm: 300_000,
        sea_level_mm: 0,
        elevations_mm: elevations,
    }
}

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let width = option_u32(&args, "--width")?.unwrap_or(PRODUCTION_DEFAULT_WIDTH);
    let height = option_u32(&args, "--height")?.unwrap_or(PRODUCTION_DEFAULT_HEIGHT);
    let seed = option_u32(&args, "--seed")?.unwrap_or(831_429);
    let warmup = option_u32(&args, "--warmup")?.unwrap_or(1);
    let repeat = option_u32(&args, "--repeat")?.unwrap_or(1);
    let idle_ms = option_u32(&args, "--idle-ms")?.unwrap_or(0);
    let dump_path = args
        .windows(2)
        .find(|window| window[0] == "--dump")
        .map(|window| window[1].clone());
    let grid =
        Grid::new(width, height, DEFAULT_RADIUS_METRES).map_err(|error| error.to_string())?;
    let physical = synthetic_field(grid, seed);
    let settings = ClimateSettings::default_for(grid);
    println!(
        "pid={} cells={} width={} height={}",
        std::process::id(),
        grid.sample_count(),
        width,
        height
    );
    if idle_ms > 0 {
        thread::sleep(Duration::from_millis(u64::from(idle_ms)));
    }
    for _ in 0..warmup {
        derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .map_err(|error| error.to_string())?;
    }
    let mut times_ms = Vec::with_capacity(repeat as usize);
    let mut last: Option<ClimateField> = None;
    for _ in 0..repeat {
        let started = Instant::now();
        let climate = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .map_err(|error| error.to_string())?;
        times_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        last = Some(climate);
    }
    let mean = if times_ms.is_empty() {
        0.0
    } else {
        times_ms.iter().sum::<f64>() / times_ms.len() as f64
    };
    let climate = last.ok_or_else(|| "no climate runs".to_string())?;
    let mut biomes = vec![0u32; BIOME_CLASS_MAX as usize + 1];
    for class in &climate.biome_class {
        if let Some(slot) = biomes.get_mut(*class as usize) {
            *slot += 1;
        }
    }
    if let Some(path) = dump_path {
        write_u32_le(
            &format!("{path}.precip"),
            &climate.precipitation_mm_per_year,
        )?;
        write_u32_le(&format!("{path}.moisture"), &climate.moisture_mm_per_year)?;
    }
    println!(
        "{{\"width\":{},\"height\":{},\"warmup\":{},\"repeat\":{},\"meanMs\":{:.3},\"transportIterations\":{},\"meanTemperatureCentiC\":{},\"meanPrecipitationMm\":{},\"wettestMm\":{},\"dominantLandBiome\":{},\"biomes\":[{}],\"runsMs\":[{}]}}",
        width,
        height,
        warmup,
        repeat,
        mean,
        climate.metrics.transport_iterations,
        climate.metrics.mean_temperature_centi_c,
        climate.metrics.mean_precipitation_mm_per_year,
        climate.metrics.wettest_cell_precipitation_mm_per_year,
        climate.metrics.dominant_land_biome,
        biomes
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(","),
        times_ms
            .iter()
            .map(|value| format!("{value:.3}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(())
}

fn write_u32_le(path: &str, values: &[u32]) -> Result<(), String> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write(path, bytes).map_err(|error| error.to_string())
}
