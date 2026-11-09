use std::collections::VecDeque;

use crate::grid::{CubeSphere, NONE};
use crate::noise::{derive_seed, hash_u01, noise_on_sphere, SeedDomain};
use crate::routing::{route, validate_routing, BasinBalance, RoutingScratch};

fn terrain(n: u32, seed: u64, ocean_fraction: f64) -> (CubeSphere, Vec<f32>, Vec<u8>) {
    let grid = CubeSphere::new(n, 6_371_000.0).unwrap();
    let noise_seed = derive_seed(seed, SeedDomain::BaseDetail);
    let mut h = vec![0.0f32; grid.cell_count()];
    let mut domain = vec![1u8; grid.cell_count()];
    for cell in 0..grid.cell_count() {
        let p = grid.position(cell as u32);
        let broad = noise_on_sphere(noise_seed, p, 4.0);
        let fine = noise_on_sphere(noise_seed ^ 0x11, p, 9.0);
        h[cell] = (broad * 900.0 + fine * 220.0) as f32;
        if hash_u01(noise_seed, cell as u64, 3) < ocean_fraction {
            domain[cell] = 0;
            h[cell] -= 400.0;
        }
    }
    (grid, h, domain)
}

fn assert_routed(n: u32, seed: u64, ocean_fraction: f64) {
    let (grid, h, domain) = terrain(n, seed, ocean_fraction);
    let mut scratch = RoutingScratch::new(grid.cell_count());
    let _stats = route(&grid, &h, &domain, None, &mut scratch, None);
    validate_routing(&scratch.receiver, &scratch.stack).unwrap();
    let mut root_area = 0.0f64;
    let mut total = 0.0f64;
    let mut roots = 0u32;
    for cell in 0..grid.cell_count() {
        total += grid.area_m2(cell as u32);
        if scratch.receiver[cell] == cell as u32 {
            roots += 1;
            root_area += scratch.area[cell];
        }
    }
    let rel = (root_area - total).abs() / total;
    assert!(rel < 1e-6, "n={n} rel={rel}");
    if ocean_fraction == 0.0 {
        assert_eq!(roots, 1, "n={n} seed={seed} roots={roots}");
    } else {
        assert!(roots >= 1);
    }
}

#[test]
fn depression_routing_on_pitted_terrain() {
    for n in [16u32, 32] {
        assert_routed(n, 3, 0.2);
        assert_routed(n, 11, 0.35);
    }
}

#[test]
fn closed_sphere_keeps_one_root() {
    for n in [16u32, 32] {
        assert_routed(n, 5, 0.0);
        assert_routed(n, 9, 0.0);
    }
}

/// Every basin pair that touches an open pit basin must get its lowest pass,
/// including links to the ocean node through lower-id ocean cells.
#[test]
fn pass_edges_match_brute_force() {
    use crate::grid::NONE;
    use crate::routing::collect_pass_edges;
    use std::collections::BTreeMap;

    let grid = CubeSphere::new(16, 6_371_000.0).unwrap();
    let seed = derive_seed(21, SeedDomain::BaseDetail);
    for with_ocean in [true, false] {
        let n = grid.cell_count();
        let mut elev = vec![0.0f32; n];
        let mut basin = vec![NONE; n];
        for cell in 0..n {
            let p = grid.position(cell as u32);
            elev[cell] = (noise_on_sphere(seed, p, 5.0) * 1000.0) as f32;
            let band = noise_on_sphere(seed ^ 0x77, p, 3.0);
            basin[cell] = if with_ocean && band < -0.25 {
                0
            } else if band > 0.55 {
                NONE
            } else {
                1 + (hash_u01(seed, (cell / 37) as u64, 9) * 12.0) as u32
            };
        }
        let ocean_node = if with_ocean { 0 } else { NONE };
        let cells: Vec<u32> = (0..n as u32)
            .filter(|&c| basin[c as usize] != NONE && basin[c as usize] != ocean_node)
            .collect();
        let mut expect: BTreeMap<(u32, u32), f32> = BTreeMap::new();
        for &c in &cells {
            for nb in grid.neighbors(c) {
                if nb == NONE {
                    continue;
                }
                let (na, nn) = (basin[c as usize], basin[nb as usize]);
                if nn == NONE || nn == na {
                    continue;
                }
                let key = (na.min(nn), na.max(nn));
                let pass = elev[c as usize].max(elev[nb as usize]);
                let slot = expect.entry(key).or_insert(pass);
                if pass < *slot {
                    *slot = pass;
                }
            }
        }
        let got: BTreeMap<(u32, u32), f32> =
            collect_pass_edges(&grid, &elev, &basin, ocean_node, &cells)
                .iter()
                .map(|e| ((e.a, e.b), e.pass))
                .collect();
        assert!(!expect.is_empty());
        assert_eq!(got, expect, "with_ocean={with_ocean}");
    }
}

#[test]
fn balance_closes_pits_and_zero_evap_closes_none() {
    let (grid, h, domain) = terrain(16, 4, 0.25);
    let n = h.len();
    let mut scratch = RoutingScratch::new(n);
    let open = route(&grid, &h, &domain, None, &mut scratch, None);
    assert_eq!(open.closed_basins, 0);
    let open_recv = scratch.receiver.clone();
    let open_stack = scratch.stack.clone();
    validate_routing(&open_recv, &open_stack).unwrap();

    let runoff = vec![0.01f32; n];
    let dry = vec![0.0f32; n];
    let zero = route(
        &grid,
        &h,
        &domain,
        None,
        &mut scratch,
        Some(&BasinBalance {
            runoff_m_yr: &runoff,
            evap_m_yr: &dry,
        }),
    );
    assert_eq!(zero.closed_basins, 0);
    assert!(zero.closed_pits.is_empty());
    assert_eq!(scratch.receiver, open_recv);
    assert_eq!(scratch.stack, open_stack);

    let wet_air = vec![1.0e6f32; n];
    let trickle = vec![1.0e-9f32; n];
    let shut = route(
        &grid,
        &h,
        &domain,
        None,
        &mut scratch,
        Some(&BasinBalance {
            runoff_m_yr: &trickle,
            evap_m_yr: &wet_air,
        }),
    );
    validate_routing(&scratch.receiver, &scratch.stack).unwrap();
    assert!(shut.closed_basins > 0, "closed {}", shut.closed_basins);
    assert_eq!(shut.closed_pits.len() as u32, shut.closed_basins);
    for &pit in &shut.closed_pits {
        assert_eq!(
            scratch.receiver[pit as usize], pit,
            "closed pit {pit} was carved"
        );
        assert_eq!(domain[pit as usize], 1);
    }
}

#[test]
fn bowl_stays_closed_until_runoff_exceeds_evaporation() {
    let (grid, h, domain, pit) = bowl();
    let n = h.len();
    let mut scratch = RoutingScratch::new(n);
    let trickle = vec![1.0e-8f32; n];
    let arid = vec![1.0e6f32; n];
    let shut = route(
        &grid,
        &h,
        &domain,
        None,
        &mut scratch,
        Some(&BasinBalance {
            runoff_m_yr: &trickle,
            evap_m_yr: &arid,
        }),
    );
    validate_routing(&scratch.receiver, &scratch.stack).unwrap();
    assert_eq!(scratch.receiver[pit as usize], pit);
    assert!(shut.closed_pits.contains(&pit));
    assert!(shut.carved < shut.pits);

    let flood = vec![5.0f32; n];
    let none = vec![0.0f32; n];
    let open = route(
        &grid,
        &h,
        &domain,
        None,
        &mut scratch,
        Some(&BasinBalance {
            runoff_m_yr: &flood,
            evap_m_yr: &none,
        }),
    );
    validate_routing(&scratch.receiver, &scratch.stack).unwrap();
    assert_eq!(open.closed_basins, 0);
    assert_ne!(scratch.receiver[pit as usize], pit);
}

fn bowl() -> (CubeSphere, Vec<f32>, Vec<u8>, u32) {
    let grid = CubeSphere::new(8, 6_371_000.0).unwrap();
    let n = grid.cell_count();
    let mut domain = vec![1u8; n];
    domain[0] = 0;
    let mut h = vec![0.0f32; n];
    let mut depth = vec![u32::MAX; n];
    let mut queue = VecDeque::new();
    depth[0] = 0;
    h[0] = -200.0;
    queue.push_back(0u32);
    while let Some(cell) = queue.pop_front() {
        for nb in grid.neighbors(cell) {
            if nb == NONE || depth[nb as usize] != u32::MAX {
                continue;
            }
            depth[nb as usize] = depth[cell as usize] + 1;
            h[nb as usize] = depth[nb as usize] as f32 * 80.0;
            queue.push_back(nb);
        }
    }
    let pit = (0..n)
        .filter(|&cell| depth[cell] >= 3 && domain[cell] == 1)
        .max_by_key(|&cell| (depth[cell], cell))
        .unwrap();
    h[pit] = -20.0;
    (grid, h, domain, pit as u32)
}
