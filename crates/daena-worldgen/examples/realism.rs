//! Realism metrics for the world engine, compared with Earth.
//!
//! Usage:
//!   realism <preset> <seed> [n]   one world
//!   realism suite [n]             Earthlike seeds 1-7, then Pangea,
//!                                 Shattered and Archipelago seed 7
//!
//! Metric definitions follow the review in `.agents/realism-check.md`:
//! land is every cell with `ocean == 0` (lakes included), every value is
//! area-weighted with `grid.area_m2`, and latitude bands use `|lat|`.
//! Earth constants and their sources are in the `earth` constants below.
use std::collections::BTreeMap;

use daena_worldgen::{generate, Biome, NoProgress, Preset, World};

const R_KM: f64 = 6371.0;
const NONE: u32 = daena_worldgen::NONE;

// Earth reference values. Sources:
// - Hypsometry: ETOPO5 tables, stevedutch.net/usefuldata/earthhypso5min.htm.
//   Land >= 2,000 m is 11.7 % with Antarctica and 7.1 % without; >= 1,000 m is
//   27 % with and 22 % without. This file uses the values without Antarctica,
//   which keeps Greenland's ice surface.
// - Lakes: HydroLAKES (Messager et al. 2016), 2.67 M km2 = 1.8 % of land;
//   Wikipedia list of lakes by area: 1 lake >= 100,000 km2 (Caspian) and 15
//   lakes >= 10,000 km2.
// - Islands: Wikipedia list of islands by area: 18 islands of 100,000 km2 to
//   3 M km2 and 57 of 10,000 to 100,000 km2.
// - Climate bands: WorldClim 2.1 10' monthly normals (1970-2000, Fick &
//   Hijmans 2017), land only, same definitions as the engine. Bands at
//   0-10 ... 60-70 degrees: coldest / annual / warmest degrees C, rain mm.
// - Koppen: same classifier on WorldClim, land north of 60 S. Af 5.7, all A
//   21.8, Aw 12.5, BWh 15.2, BWk 3.7, all B 30.5, C 13.8, D 26.7 %.
// - Coast: WorldClim same definitions, 15-35 degrees, coastal strip at most
//   500 km from the sea on landmasses at least 1,500 km wide: west 284 mm,
//   east 870 mm, east/west ratio 3.07; arid (B) share west/middle/east
//   78/58/41 %.
// - Global land precipitation is about 800 mm/yr (review text).
// - Shelf: Harris et al. 2014, about 27 M km2 = 9 % of ocean; passive-margin
//   width 88 km, active 31 km.
const EARTH_LAND_FRAC: f64 = 0.292;
const EARTH_P_LAND_MM: f64 = 800.0;
const EARTH_KOPPEN: &[(&str, f64)] = &[
    ("Af", 5.7),
    ("all A", 21.8),
    ("Aw", 12.5),
    ("BWh", 15.2),
    ("BWk", 3.7),
    ("all B", 30.5),
    ("C", 13.8),
    ("D", 26.7),
];
const EARTH_BANDS: &[(f64, f64, f64, f64)] = &[
    // lo, coldest, annual, warmest, rain (only one value here; see bands()).
    (0.0, 23.8, 25.0, 26.1),
    (10.0, 20.6, 24.9, 28.3),
    (20.0, 13.7, 21.9, 28.7),
    (30.0, 2.3, 13.1, 23.4),
    (40.0, -8.1, 6.6, 20.3),
    (50.0, -16.2, 0.3, 16.0),
    (60.0, -25.8, -9.1, 9.6),
];
const EARTH_BAND_RAIN: &[f64] = &[1842.0, 972.0, 576.0, 552.0, 573.0, 620.0, 423.0];
const EARTH_LAKE_SHARE: f64 = 1.8;
const EARTH_LAKES_100K: usize = 1;
const EARTH_LAKES_10K: usize = 15;
const EARTH_ISLANDS_100K: usize = 18;
const EARTH_ISLANDS_10K: usize = 57;
const EARTH_HYPSO_1000: f64 = 22.0;
const EARTH_HYPSO_2000: f64 = 7.1;
const EARTH_COAST_EAST_WEST: f64 = 3.07;
const EARTH_COAST_WEST_MM: f64 = 284.0;
const EARTH_COAST_EAST_MM: f64 = 870.0;
const EARTH_ARID_THIRDS: [f64; 3] = [78.0, 58.0, 41.0];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: realism <preset> <seed> [n] | realism suite [n]");
        std::process::exit(2);
    }
    if args[1] == "suite" {
        let n: u32 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(512);
        let runs = [
            ("earthlike", 1u64),
            ("earthlike", 2),
            ("earthlike", 3),
            ("earthlike", 4),
            ("earthlike", 5),
            ("earthlike", 6),
            ("earthlike", 7),
            ("pangea", 7),
            ("shattered", 7),
            ("archipelago", 7),
        ];
        for (preset, seed) in runs {
            let world = run(preset, seed, n);
            print_metrics(preset, seed, n, &world);
        }
        return;
    }
    let preset = Preset::parse(&args[1]).expect("preset");
    let seed: u64 = args[2].parse().expect("seed");
    let n: u32 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(512);
    let world = run(preset.as_str(), seed, n);
    print_metrics(preset.as_str(), seed, n, &world);
}

fn run(preset: &str, seed: u64, n: u32) -> World {
    let mut settings = Preset::parse(preset).expect("preset").settings();
    settings.seed = seed;
    settings.resolution = n;
    generate(&settings, &mut NoProgress).expect("generate")
}

fn print_metrics(preset: &str, seed: u64, n: u32, w: &World) {
    let m = Metrics::compute(w);
    println!(
        "== {preset} seed {seed} n {n} ({} ms)",
        w.timings_total_ms()
    );
    println!("layout: {:?}", w.tectonics.resolved_layout);
    println!(
        "land: {:.2} Mkm2  {:.3} fraction   Earth {:.3}",
        m.land_area / 1e6,
        m.land_frac,
        EARTH_LAND_FRAC
    );
    println!(
        "P land: {:.0} mm   Earth {:.0}",
        m.p_land_mm, EARTH_P_LAND_MM
    );
    println!("bands: lo  land%  Tcold  Tann  Twarm  P   (Earth Tcold/Tann/Twarm/P)");
    for (i, b) in m.bands.iter().enumerate() {
        let earth = EARTH_BANDS.get(i);
        let earth_rain = EARTH_BAND_RAIN.get(i);
        println!(
            "{:>2}-{:<2} {:>5.1}% {:>6.1} {:>5.1} {:>5.1} {:>5.0}   ({:>6.1}/{:>5.1}/{:>5.1}/{:>4.0})",
            b.lo,
            b.hi,
            100.0 * b.land_share,
            b.t_cold,
            b.t_annual,
            b.t_warm,
            b.p,
            earth.map(|e| e.1).unwrap_or(f64::NAN),
            earth.map(|e| e.2).unwrap_or(f64::NAN),
            earth.map(|e| e.3).unwrap_or(f64::NAN),
            earth_rain.copied().unwrap_or(f64::NAN),
        );
    }
    print!("koppen %:");
    for (k, v) in &m.koppen {
        print!(" {k}={v:.1}");
    }
    print!("   Earth:");
    for (k, v) in EARTH_KOPPEN {
        print!(" {k}={v:.1}");
    }
    println!();
    print!("biome %:");
    for (k, v) in &m.biomes {
        print!(" {k}={v:.1}");
    }
    println!();
    println!(
        "coldest<0 at 30-40: {:.0}%  Earth 33%   Tcold 20-30 {:.1} Earth 13.7  Tcold 30-40 {:.1} Earth 2.3",
        m.cold_below_zero_3040, m.t_cold_2030, m.t_cold_3040
    );
    println!(
        "hyps: mean {:.0} m median {:.0} | >=500 {:.1}% >=1000 {:.1}% (Earth {:.0}) >=2000 {:.1}% (Earth {:.0}) >=3000 {:.1}% | ice {:.2} Mkm2",
        m.mean_elev,
        m.median_elev,
        m.above_500,
        m.above_1000,
        EARTH_HYPSO_1000,
        m.above_2000,
        EARTH_HYPSO_2000,
        m.above_3000,
        m.ice_area / 1e6
    );
    println!(
        "lakes: count {} | {:.1}% of land (Earth {:.1}) fresh {:.1} saline {:.1} saltflat {:.1} | >=10k {} (Earth {}) >=100k {} (Earth {}) | largest {:.0} km2 | max mean depth {:.0} m | deep>=100m {}",
        m.lakes.len(),
        m.lake_share,
        EARTH_LAKE_SHARE,
        m.fresh_share,
        m.saline_share,
        m.saltflat_share,
        m.lakes_10k,
        EARTH_LAKES_10K,
        m.lakes_100k,
        EARTH_LAKES_100K,
        m.largest_lake_km2,
        m.max_mean_depth,
        m.deep_lakes
    );
    if let Some(level) = w.stats.levels.last() {
        println!(
            "breach rules: drained {} | kept subsiding {} glacial {} arid {}",
            level.breached_basins,
            level.kept_subsiding_basins,
            level.kept_glacial_basins,
            level.kept_arid_basins
        );
    }
    if std::env::var_os("REALISM_LAKES").is_some() {
        let mut rows: Vec<&LakeRow> = m.lakes.iter().collect();
        rows.sort_by(|a, b| b.area_km2.partial_cmp(&a.area_km2).unwrap());
        for row in rows.iter().take(12) {
            println!(
                "  lake {:>9.0} km2  L {:>5.0}  L2/A {:>5.1}  mean {:>4.0} max {:>5.0}  rift {:.2}",
                row.area_km2,
                row.length_km,
                row.elong,
                row.mean_depth,
                row.max_depth,
                row.rift_frac
            );
        }
    }
    println!(
        "landmasses: total {} | >=3M {} | 100k-3M {} (Earth {}) | 10k-100k {} (Earth {}) | largest share {:.2}",
        m.landmasses,
        m.continents,
        m.islands_100k,
        EARTH_ISLANDS_100K,
        m.islands_10k,
        EARTH_ISLANDS_10K,
        m.largest_share
    );
    println!(
        "coast 15-35: W/M/E P {:.0}/{:.0}/{:.0} (Earth {:.0}/-/{:.0})  B% {:.0}/{:.0}/{:.0} (Earth {:.0}/{:.0}/{:.0})  coastal E/W ratio {:.2} (Earth {:.2})",
        m.coast_p[0],
        m.coast_p[1],
        m.coast_p[2],
        EARTH_COAST_WEST_MM,
        EARTH_COAST_EAST_MM,
        m.arid_third[0],
        m.arid_third[1],
        m.arid_third[2],
        EARTH_ARID_THIRDS[0],
        EARTH_ARID_THIRDS[1],
        EARTH_ARID_THIRDS[2],
        m.coast_ratio,
        EARTH_COAST_EAST_WEST
    );
    println!(
        "rifts: longest trough {:.0} km (Earth rift lakes <= ~1000 km) | rift lakes {} longest {:.0} km (target <= 700) | L2/A range {:.1}-{:.1} (target 8-20) | trough area {:.0} km2",
        m.longest_rift_km,
        m.rift_lakes,
        m.longest_rift_lake_km,
        m.rift_lake_elong_min,
        m.rift_lake_elong_max,
        m.rift_area_km2
    );
    println!(
        "shelf: {:.1}% of ocean (Earth ~9) | width km p50 {:.0} p90 {:.0} (Earth passive 88, active 31) | max lake length {:.0} km | max L2/A {:.1}",
        m.shelf_share,
        m.shelf_p50,
        m.shelf_p90,
        m.max_lake_km,
        m.max_lake_elong
    );
    println!(
        "hashes: elevation {:016x} hydrology {:016x}",
        w.elevation_hash(),
        w.hydrology_hash()
    );
}

struct Metrics {
    land_area: f64,
    land_frac: f64,
    p_land_mm: f64,
    bands: Vec<Band>,
    koppen: BTreeMap<&'static str, f64>,
    biomes: BTreeMap<String, f64>,
    cold_below_zero_3040: f64,
    t_cold_2030: f64,
    t_cold_3040: f64,
    mean_elev: f64,
    median_elev: f64,
    above_500: f64,
    above_1000: f64,
    above_2000: f64,
    above_3000: f64,
    ice_area: f64,
    lakes: Vec<LakeRow>,
    lake_share: f64,
    fresh_share: f64,
    saline_share: f64,
    saltflat_share: f64,
    lakes_10k: usize,
    lakes_100k: usize,
    largest_lake_km2: f64,
    max_mean_depth: f64,
    deep_lakes: usize,
    landmasses: usize,
    continents: usize,
    islands_100k: usize,
    islands_10k: usize,
    largest_share: f64,
    coast_p: [f64; 3],
    arid_third: [f64; 3],
    coast_ratio: f64,
    longest_rift_km: f64,
    rift_lakes: usize,
    longest_rift_lake_km: f64,
    rift_lake_elong_min: f64,
    rift_lake_elong_max: f64,
    rift_area_km2: f64,
    shelf_share: f64,
    shelf_p50: f64,
    shelf_p90: f64,
    max_lake_km: f64,
    max_lake_elong: f64,
}

struct Band {
    lo: f64,
    hi: f64,
    land_share: f64,
    t_cold: f64,
    t_annual: f64,
    t_warm: f64,
    p: f64,
}

struct LakeRow {
    area_km2: f64,
    length_km: f64,
    elong: f64,
    mean_depth: f64,
    max_depth: f64,
    rift_frac: f64,
}

impl Metrics {
    fn compute(w: &World) -> Self {
        let g = &w.grid;
        let nc = g.cell_count();
        let cl = &w.climate;
        let hy = &w.hydrology;
        let area_km2: Vec<f64> = (0..nc).map(|c| g.area_m2(c as u32) / 1e6).collect();
        let lat: Vec<f64> = (0..nc)
            .map(|c| g.lat_lon(c as u32).0.to_degrees())
            .collect();
        let land: Vec<bool> = (0..nc).map(|c| w.ocean[c] == 0).collect();
        let ice_sheet: Vec<bool> = (0..nc).map(|c| hy.ice_cover[c] == 2).collect();
        let land_area: f64 = (0..nc).filter(|&c| land[c]).map(|c| area_km2[c]).sum();
        let total_area: f64 = area_km2.iter().sum();
        let wmean = |f: &dyn Fn(usize) -> Option<f64>, lo: f64, hi: f64, hemi: i32| -> (f64, f64) {
            let mut s = 0.0;
            let mut a = 0.0;
            for c in 0..nc {
                if !land[c] {
                    continue;
                }
                let l = lat[c];
                let al = l.abs();
                if al < lo || al >= hi {
                    continue;
                }
                if (hemi > 0 && l < 0.0) || (hemi < 0 && l > 0.0) {
                    continue;
                }
                if let Some(v) = f(c) {
                    s += v * area_km2[c];
                    a += area_km2[c];
                }
            }
            (if a > 0.0 { s / a } else { f64::NAN }, a)
        };

        let mut bands = Vec::new();
        for b in 0..9 {
            let lo = b as f64 * 10.0;
            let hi = lo + 10.0;
            let (t_cold, a) = wmean(&|c| Some(cl.t_cold_c[c] as f64), lo, hi, 0);
            let (t_annual, _) = wmean(&|c| Some(cl.t_annual_c[c] as f64), lo, hi, 0);
            let (t_warm, _) = wmean(&|c| Some(cl.t_warm_c[c] as f64), lo, hi, 0);
            let (p, _) = wmean(&|c| Some(cl.p_annual_mm[c] as f64), lo, hi, 0);
            bands.push(Band {
                lo,
                hi,
                land_share: a / land_area,
                t_cold,
                t_annual,
                t_warm,
                p,
            });
        }

        // Koppen (Peel 2007 rules, same as the review); the summer share
        // comes from the coarse monthly field that contains each fine cell.
        let cf = &w.climate_field;
        let ncc = cf.grid.cell_count();
        let coarse_of: Vec<u32> = (0..nc)
            .map(|c| cf.grid.cell_at(g.position(c as u32)))
            .collect();
        let mut koppen = BTreeMap::<&'static str, f64>::new();
        let mut biome = BTreeMap::<String, f64>::new();
        let mut kclass = vec![""; nc];
        for c in 0..nc {
            if !land[c] {
                continue;
            }
            let mat = cl.t_annual_c[c] as f64;
            let thot = cl.t_warm_c[c] as f64;
            let tcold = cl.t_cold_c[c] as f64;
            let map = cl.p_annual_mm[c] as f64;
            let pdry = cl.p_dry_month_mm[c] as f64;
            let cc = coarse_of[c] as usize;
            let mut ps = 0.0;
            let mut pt = 0.0;
            for m in 0..12 {
                let v = cf.p_mm[m * ncc + cc] as f64;
                pt += v;
                let nh_summer = (3..=8).contains(&m);
                if (lat[c] >= 0.0) == nh_summer {
                    ps += v;
                }
            }
            let fs = if pt > 0.0 { ps / pt } else { 0.5 };
            let pth = if fs >= 0.7 {
                2.0 * mat + 28.0
            } else if fs <= 0.3 {
                2.0 * mat
            } else {
                2.0 * mat + 14.0
            };
            let k = if thot < 10.0 {
                "E"
            } else if map < 10.0 * pth.max(0.0) {
                match (map < 5.0 * pth, mat >= 18.0) {
                    (true, true) => "BWh",
                    (true, false) => "BWk",
                    (false, true) => "BSh",
                    (false, false) => "BSk",
                }
            } else if tcold >= 18.0 {
                if pdry >= 60.0 {
                    "Af"
                } else if pdry >= 100.0 - map / 25.0 {
                    "Am"
                } else {
                    "Aw"
                }
            } else if tcold > 0.0 {
                "C"
            } else {
                "D"
            };
            kclass[c] = k;
            *koppen.entry(k).or_default() += area_km2[c];
            let b = Biome::from_u8(w.biome[c]).unwrap();
            *biome.entry(format!("{:?}", b)).or_default() += area_km2[c];
        }
        for v in koppen.values_mut() {
            *v = 100.0 * *v / land_area;
        }
        for v in biome.values_mut() {
            *v = 100.0 * *v / land_area;
        }

        let (cold_below_zero_3040, _) = wmean(
            &|c| Some(if cl.t_cold_c[c] < 0.0 { 1.0 } else { 0.0 }),
            30.0,
            40.0,
            0,
        );
        let (t_cold_2030, _) = wmean(&|c| Some(cl.t_cold_c[c] as f64), 20.0, 30.0, 0);
        let (t_cold_3040, _) = wmean(&|c| Some(cl.t_cold_c[c] as f64), 30.0, 40.0, 0);
        let (p_land_mm, _) = wmean(&|c| Some(cl.p_annual_mm[c] as f64), 0.0, 91.0, 0);

        // Hypsometry.
        let mut above = [0.0f64; 4];
        let mut elev: Vec<(f32, f64)> = Vec::new();
        let mut mean = 0.0;
        for c in 0..nc {
            if !land[c] {
                continue;
            }
            let h = w.elevation_m[c];
            elev.push((h, area_km2[c]));
            mean += h as f64 * area_km2[c];
            for (i, t) in [500.0, 1000.0, 2000.0, 3000.0].iter().enumerate() {
                if h as f64 >= *t {
                    above[i] += area_km2[c];
                }
            }
        }
        elev.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut acc = 0.0;
        let mut med = 0.0f64;
        for (h, a) in &elev {
            acc += a;
            if acc >= land_area / 2.0 {
                med = f64::from(*h);
                break;
            }
        }
        let ice_area: f64 = (0..nc).filter(|&c| ice_sheet[c]).map(|c| area_km2[c]).sum();

        // Lakes.
        let mut lake_cells: Vec<Vec<u32>> = vec![Vec::new(); hy.lakes.len()];
        for c in 0..nc {
            if hy.lake_id[c] != NONE {
                lake_cells[hy.lake_id[c] as usize].push(c as u32);
            }
        }
        let gc = |a: u32, b: u32| -> f64 {
            let p = g.position(a);
            let q = g.position(b);
            (p[0] * q[0] + p[1] * q[1] + p[2] * q[2])
                .clamp(-1.0, 1.0)
                .acos()
                * R_KM
        };
        let mut la = [0.0f64; 3];
        for l in &hy.lakes {
            la[l.kind as usize] += l.area_m2 / 1e6;
        }
        let tot_lake: f64 = la.iter().sum();
        let mut lakes = Vec::new();
        let mut max_mean_depth = 0.0f64;
        let mut deep_lakes = 0;
        for (i, l) in hy.lakes.iter().enumerate() {
            let cells = &lake_cells[i];
            let a_km2 = l.area_m2 / 1e6;
            let mean_depth = if a_km2 > 0.0 {
                l.volume_m3 / 1e6 / a_km2
            } else {
                0.0
            };
            max_mean_depth = max_mean_depth.max(mean_depth);
            if mean_depth >= 100.0 {
                deep_lakes += 1;
            }
            if cells.is_empty() || a_km2 < 5000.0 {
                if a_km2 >= 5000.0 {
                    lakes.push(LakeRow {
                        area_km2: a_km2,
                        length_km: f64::NAN,
                        elong: f64::NAN,
                        mean_depth,
                        max_depth: l.level_m - f64::from(w.elevation_m[l.minimum_cell as usize]),
                        rift_frac: 0.0,
                    });
                }
                continue;
            }
            let far = |from: u32| -> (u32, f64) {
                let mut best = (from, 0.0);
                for &c in cells {
                    let d = gc(from, c);
                    if d > best.1 {
                        best = (c, d);
                    }
                }
                best
            };
            let (a1, _) = far(cells[0]);
            let (_, len) = far(a1);
            let rift_frac = cells
                .iter()
                .filter(|&&c| w.uplift_m_yr[c as usize] < 0.0)
                .count() as f64
                / cells.len() as f64;
            lakes.push(LakeRow {
                area_km2: a_km2,
                length_km: len,
                elong: len * len / a_km2,
                mean_depth,
                max_depth: l.level_m - f64::from(w.elevation_m[l.minimum_cell as usize]),
                rift_frac,
            });
        }
        let largest_lake_km2 = lakes.iter().map(|l| l.area_km2).fold(0.0, f64::max);
        let max_lake_km = lakes.iter().map(|l| l.length_km).fold(0.0, f64::max);
        let max_lake_elong = lakes.iter().map(|l| l.elong).fold(0.0, f64::max);

        // 4-connected land components.
        let axis: [usize; 4] = [0, 2, 4, 6];
        let mut comp = vec![NONE; nc];
        let mut comps: Vec<f64> = Vec::new();
        for s in 0..nc {
            if !land[s] || comp[s] != NONE {
                continue;
            }
            let id = comps.len() as u32;
            comps.push(0.0);
            let mut q = std::collections::VecDeque::new();
            comp[s] = id;
            q.push_back(s);
            while let Some(c) = q.pop_front() {
                comps[id as usize] += area_km2[c];
                let nb = g.neighbors(c as u32);
                for &k in &axis {
                    let m = nb[k];
                    if m != NONE && land[m as usize] && comp[m as usize] == NONE {
                        comp[m as usize] = id;
                        q.push_back(m as usize);
                    }
                }
            }
        }

        comps.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let count = |lo: f64, hi: f64| comps.iter().filter(|&&a| a >= lo && a < hi).count();
        let largest_share = comps.first().copied().unwrap_or(0.0) / land_area;

        // Continental position and coast at 15-35 degrees.
        let step_km = g.cell_size_m() / 1000.0 * 0.7;
        let dist_to_ocean = |c: usize, dir: f64| -> f64 {
            let (la, lo) = g.lat_lon(c as u32);
            let coslat = la.cos().max(1e-3);
            let dlon = step_km / (R_KM * coslat) * dir;
            let mut k = 1;
            loop {
                let d = k as f64 * step_km;
                if d > 8000.0 {
                    return 8000.0;
                }
                let l2 = lo + dlon * k as f64;
                let p = [la.cos() * l2.cos(), la.cos() * l2.sin(), la.sin()];
                let cc = g.cell_at(p) as usize;
                if !land[cc] {
                    return d;
                }
                k += 1;
            }
        };
        let mut bins = [[0.0f64; 4]; 3];
        let mut coast = [[0.0f64; 2]; 2];
        for c in 0..nc {
            if !land[c] {
                continue;
            }
            let al = lat[c].abs();
            if !(15.0..35.0).contains(&al) {
                continue;
            }
            let dw = dist_to_ocean(c, -1.0);
            let de = dist_to_ocean(c, 1.0);
            let x = dw / (dw + de);
            let bi = if x < 1.0 / 3.0 {
                0
            } else if x < 2.0 / 3.0 {
                1
            } else {
                2
            };
            let a = area_km2[c];
            let p = cl.p_annual_mm[c] as f64;
            let isb = kclass[c].starts_with('B');
            bins[bi][0] += p * a;
            bins[bi][1] += a;
            bins[bi][2] += if isb { a } else { 0.0 };
            if dw.min(de) <= 500.0 && (dw + de) >= 1500.0 {
                let side = usize::from(dw >= de);
                coast[side][0] += p * a;
                coast[side][1] += a;
            }
        }
        let coast_p = [
            bins[0][0] / bins[0][1],
            bins[1][0] / bins[1][1],
            bins[2][0] / bins[2][1],
        ];
        let arid_third = [
            100.0 * bins[0][2] / bins[0][1],
            100.0 * bins[1][2] / bins[1][1],
            100.0 * bins[2][2] / bins[2][1],
        ];
        let west = coast[0][0] / coast[0][1];
        let east = coast[1][0] / coast[1][1];
        let coast_ratio = east / west;

        // Rifts: continental cells with the engine's negative uplift term.
        let rift: Vec<bool> = (0..nc)
            .map(|c| w.crust[c] == 1 && w.uplift_m_yr[c] < 0.0)
            .collect();
        let mut seen = vec![false; nc];
        let mut longest_rift_km = 0.0f64;
        let mut rift_area_km2 = 0.0;
        for s0 in 0..nc {
            if !rift[s0] || seen[s0] {
                continue;
            }
            let mut cells = Vec::new();
            let mut q = std::collections::VecDeque::new();
            seen[s0] = true;
            q.push_back(s0);
            while let Some(c) = q.pop_front() {
                cells.push(c as u32);
                for m in g.neighbors(c as u32) {
                    if m != NONE && rift[m as usize] && !seen[m as usize] {
                        seen[m as usize] = true;
                        q.push_back(m as usize);
                    }
                }
            }
            let a: f64 = cells.iter().map(|&c| area_km2[c as usize]).sum();
            if a < 5000.0 {
                continue;
            }
            rift_area_km2 += a;
            let far = |from: u32| -> (u32, f64) {
                let mut best = (from, 0.0);
                for &c in &cells {
                    let d = gc(from, c);
                    if d > best.1 {
                        best = (c, d);
                    }
                }
                best
            };
            let (a1, _) = far(cells[0]);
            let (_, len) = far(a1);
            longest_rift_km = longest_rift_km.max(len);
        }

        // Rift lakes: at least half their cells in the negative-uplift field.
        let mut rift_lake_count = 0usize;
        let mut longest_rift_lake_km = 0.0f64;
        let mut elong: Vec<f64> = Vec::new();
        for (i, l) in hy.lakes.iter().enumerate() {
            let cells = &lake_cells[i];
            let a_km2 = l.area_m2 / 1e6;
            if cells.len() < 4 || a_km2 < 1.0e4 {
                continue;
            }
            let neg = cells
                .iter()
                .filter(|&&c| w.uplift_m_yr[c as usize] < 0.0)
                .count() as f64
                / cells.len() as f64;
            if neg < 0.5 {
                continue;
            }
            rift_lake_count += 1;
            let far = |from: u32| -> (u32, f64) {
                let mut best = (from, 0.0);
                for &c in cells {
                    let d = gc(from, c);
                    if d > best.1 {
                        best = (c, d);
                    }
                }
                best
            };
            let (a1, _) = far(cells[0]);
            let (_, len) = far(a1);
            longest_rift_lake_km = longest_rift_lake_km.max(len);
            elong.push(len * len / a_km2);
        }
        elong.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Shelf.
        let ocean_area = total_area - land_area;
        let shelf: f64 = (0..nc)
            .filter(|&c| !land[c] && w.elevation_m[c] > -200.0)
            .map(|c| area_km2[c])
            .sum();
        let mut hop = vec![u32::MAX; nc];
        let mut q = std::collections::VecDeque::new();
        for c in 0..nc {
            if !land[c] && w.elevation_m[c] <= -200.0 {
                hop[c] = 0;
                q.push_back(c);
            }
        }
        while let Some(c) = q.pop_front() {
            let nb = g.neighbors(c as u32);
            for &k in &axis {
                let m = nb[k];
                if m != NONE && !land[m as usize] && hop[m as usize] == u32::MAX {
                    hop[m as usize] = hop[c] + 1;
                    q.push_back(m as usize);
                }
            }
        }
        let csz = g.cell_size_m() / 1000.0;
        let mut widths: Vec<f64> = Vec::new();
        for c in 0..nc {
            if land[c] || hop[c] == u32::MAX {
                continue;
            }
            let nb = g.neighbors(c as u32);
            if axis.iter().any(|&k| nb[k] != NONE && land[nb[k] as usize]) {
                widths.push(hop[c] as f64 * csz);
            }
        }
        widths.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let pct = |p: f64| -> f64 {
            if widths.is_empty() {
                f64::NAN
            } else {
                widths[((widths.len() as f64 - 1.0) * p) as usize]
            }
        };

        Self {
            land_area,
            land_frac: land_area / total_area,
            p_land_mm,
            bands,
            koppen,
            biomes: biome,
            cold_below_zero_3040: 100.0 * cold_below_zero_3040,
            t_cold_2030,
            t_cold_3040,
            mean_elev: mean / land_area,
            median_elev: med,
            above_500: 100.0 * above[0] / land_area,
            above_1000: 100.0 * above[1] / land_area,
            above_2000: 100.0 * above[2] / land_area,
            above_3000: 100.0 * above[3] / land_area,
            ice_area,
            lakes,
            lake_share: 100.0 * tot_lake / land_area,
            fresh_share: 100.0 * la[0] / land_area,
            saline_share: 100.0 * la[1] / land_area,
            saltflat_share: 100.0 * la[2] / land_area,
            lakes_10k: hy.lakes.iter().filter(|l| l.area_m2 / 1e6 >= 1e4).count(),
            lakes_100k: hy.lakes.iter().filter(|l| l.area_m2 / 1e6 >= 1e5).count(),
            largest_lake_km2,
            max_mean_depth,
            deep_lakes,
            landmasses: comps.len(),
            continents: count(3e6, f64::INFINITY),
            islands_100k: count(1e5, 3e6),
            islands_10k: count(1e4, 1e5),
            largest_share,
            coast_p,
            arid_third,
            coast_ratio,
            longest_rift_km,
            rift_lakes: rift_lake_count,
            longest_rift_lake_km,
            rift_lake_elong_min: elong.first().copied().unwrap_or(f64::NAN),
            rift_lake_elong_max: elong.last().copied().unwrap_or(f64::NAN),
            rift_area_km2,
            shelf_share: 100.0 * shelf / ocean_area,
            shelf_p50: pct(0.5),
            shelf_p90: pct(0.9),
            max_lake_km,
            max_lake_elong,
        }
    }
}

trait TimingsTotal {
    fn timings_total_ms(&self) -> f64;
}

impl TimingsTotal for World {
    fn timings_total_ms(&self) -> f64 {
        self.timings
            .iter()
            .find(|(name, _)| *name == "total")
            .map(|(_, ms)| *ms)
            .unwrap_or(0.0)
    }
}
