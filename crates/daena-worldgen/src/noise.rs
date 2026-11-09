//! Seeded hashes and 3D gradient noise.
//!
//! Every random decision in the crate goes through [`derive_seed`] and a
//! [`SeedDomain`], so changing one use does not shift the others.

/// Distinct 64-bit tag for each random use in the generator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedDomain {
    /// Plate-site positions.
    Plates,
    /// Plate angular weights.
    PlateWeights,
    /// Euler pole axes.
    EulerPoles,
    /// Plate angular speeds.
    AngularSpeed,
    /// Which oceanic plate subducts.
    SubductionRank,
    /// Domain warp of the plate Voronoi field.
    PlateWarp,
    /// Continent blob centres, counts and radii.
    Continents,
    /// Warp applied before continent distance tests.
    ContinentWarp,
    /// Broad continent fBm.
    ContinentFbm,
    /// Mixed-layout choice.
    LayoutChoice,
    /// Hotspot origins and counts.
    Hotspots,
    /// Hotspot trail length and spacing.
    HotspotTrail,
    /// Island-arc ridged noise.
    ArcRidged,
    /// Lithology / erodibility fBm.
    Lithology,
    /// Fine uplift added above the tectonic grid.
    UpliftDetail,
    /// Fine base elevation added above the tectonic grid.
    BaseDetail,
    /// Per-level initial roughness.
    Roughness,
    /// Along-strike breakup of uplift belts.
    UpliftModulation,
    /// Transform-fault ridged noise.
    TransformRidged,
    /// Worn interior ranges.
    OldOrogen,
    /// Background continental uplift.
    BackgroundUplift,
    /// Continental base fBm.
    ContinentalSurface,
    /// Ocean-ocean arc volcano lattice: site occupancy and jitter.
    ArcVolcano,
}

impl SeedDomain {
    /// Stable tag mixed into the world seed.
    pub fn tag(self) -> u64 {
        match self {
            Self::Plates => 0x1001,
            Self::PlateWeights => 0x1002,
            Self::EulerPoles => 0x1003,
            Self::AngularSpeed => 0x1004,
            Self::SubductionRank => 0x1005,
            Self::PlateWarp => 0x1006,
            Self::Continents => 0x1007,
            Self::ContinentWarp => 0x1008,
            Self::ContinentFbm => 0x1009,
            Self::LayoutChoice => 0x100A,
            Self::Hotspots => 0x100B,
            Self::HotspotTrail => 0x100C,
            Self::ArcRidged => 0x100D,
            Self::Lithology => 0x100E,
            Self::UpliftDetail => 0x100F,
            Self::BaseDetail => 0x1010,
            Self::Roughness => 0x1011,
            Self::UpliftModulation => 0x1012,
            Self::TransformRidged => 0x1013,
            Self::OldOrogen => 0x1014,
            Self::BackgroundUplift => 0x1015,
            Self::ContinentalSurface => 0x1016,
            Self::ArcVolcano => 0x1017,
        }
    }
}

/// SplitMix64 finalizer. The input is mixed; the function is pure.
pub fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hash a seed and three lattice coordinates.
pub fn hash3(seed: u64, x: i64, y: i64, z: i64) -> u64 {
    let mut h = seed ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h = splitmix64(h);
    h ^= (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = splitmix64(h);
    h ^= (z as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    splitmix64(h)
}

/// Domain-separated seed. All random decisions go through this.
pub fn derive_seed(seed: u64, domain: SeedDomain) -> u64 {
    splitmix64(seed ^ domain.tag().wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Extra salt for octaves and levels. Still a pure function of `seed`.
pub fn derive_seed_salt(seed: u64, domain: SeedDomain, salt: u64) -> u64 {
    splitmix64(derive_seed(seed, domain) ^ salt.wrapping_mul(0xBF58_476D_1CE4_E5B9))
}

/// Uniform value in `[0, 1)` from two integer indices.
pub fn hash_u01(seed: u64, a: u64, b: u64) -> f64 {
    let h = splitmix64(
        seed ^ a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xBF58_476D_1CE4_E5B9),
    );
    (h >> 11) as f64 * (1.0 / ((1u64 << 53) as f64))
}

/// Uniform unit vector. Index `i` selects the draw.
pub fn random_unit(seed: u64, i: u64) -> [f64; 3] {
    let z = hash_u01(seed, i, 1).mul_add(2.0, -1.0);
    let phi = hash_u01(seed, i, 2) * std::f64::consts::TAU;
    let r = (1.0 - z * z).max(0.0).sqrt();
    [r * phi.cos(), r * phi.sin(), z]
}

const INV_SQRT2: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// Twelve cube-edge gradients, stored unit length.
const GRAD3: [[f64; 3]; 12] = [
    [INV_SQRT2, INV_SQRT2, 0.0],
    [-INV_SQRT2, INV_SQRT2, 0.0],
    [INV_SQRT2, -INV_SQRT2, 0.0],
    [-INV_SQRT2, -INV_SQRT2, 0.0],
    [INV_SQRT2, 0.0, INV_SQRT2],
    [-INV_SQRT2, 0.0, INV_SQRT2],
    [INV_SQRT2, 0.0, -INV_SQRT2],
    [-INV_SQRT2, 0.0, -INV_SQRT2],
    [0.0, INV_SQRT2, INV_SQRT2],
    [0.0, -INV_SQRT2, INV_SQRT2],
    [0.0, INV_SQRT2, -INV_SQRT2],
    [0.0, -INV_SQRT2, -INV_SQRT2],
];

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn grad_dot(seed: u64, ix: i64, iy: i64, iz: i64, x: f64, y: f64, z: f64) -> f64 {
    let g = GRAD3[(hash3(seed, ix, iy, iz) % 12) as usize];
    g[0] * x + g[1] * y + g[2] * z
}

/// 3D Perlin-style gradient noise at `p`, roughly in `[-1, 1]`.
pub fn gradient_noise(seed: u64, p: [f64; 3]) -> f64 {
    let x0 = p[0].floor() as i64;
    let y0 = p[1].floor() as i64;
    let z0 = p[2].floor() as i64;
    let xf = p[0] - x0 as f64;
    let yf = p[1] - y0 as f64;
    let zf = p[2] - z0 as f64;
    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);
    let n000 = grad_dot(seed, x0, y0, z0, xf, yf, zf);
    let n100 = grad_dot(seed, x0 + 1, y0, z0, xf - 1.0, yf, zf);
    let n010 = grad_dot(seed, x0, y0 + 1, z0, xf, yf - 1.0, zf);
    let n110 = grad_dot(seed, x0 + 1, y0 + 1, z0, xf - 1.0, yf - 1.0, zf);
    let n001 = grad_dot(seed, x0, y0, z0 + 1, xf, yf, zf - 1.0);
    let n101 = grad_dot(seed, x0 + 1, y0, z0 + 1, xf - 1.0, yf, zf - 1.0);
    let n011 = grad_dot(seed, x0, y0 + 1, z0 + 1, xf, yf - 1.0, zf - 1.0);
    let n111 = grad_dot(seed, x0 + 1, y0 + 1, z0 + 1, xf - 1.0, yf - 1.0, zf - 1.0);
    let nx00 = n000 + (n100 - n000) * u;
    let nx10 = n010 + (n110 - n010) * u;
    let nx01 = n001 + (n101 - n001) * u;
    let nx11 = n011 + (n111 - n011) * u;
    let nxy0 = nx00 + (nx10 - nx00) * v;
    let nxy1 = nx01 + (nx11 - nx01) * v;
    // Unit gradients peak a bit under 1 after quintic interpolation.
    (nxy0 + (nxy1 - nxy0) * w) * 1.15
}

/// Gradient noise of a unit direction scaled by `frequency`.
pub fn noise_on_sphere(seed: u64, p: [f64; 3], frequency: f64) -> f64 {
    gradient_noise(seed, [p[0] * frequency, p[1] * frequency, p[2] * frequency])
}

/// Fractional Brownian motion. `lacunarity` defaults to 2 at call sites that pass it.
pub fn fbm(
    seed: u64,
    p: [f64; 3],
    base_frequency: f64,
    octaves: u32,
    lacunarity: f64,
    gain: f64,
) -> f64 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut freq = base_frequency;
    let mut norm = 0.0;
    for octave in 0..octaves {
        let s = splitmix64(seed ^ (octave as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        sum += amp * noise_on_sphere(s, p, freq);
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    if norm == 0.0 {
        0.0
    } else {
        sum / norm
    }
}

/// fBm divided by `1 / (1 - gain)`, not by the octaves actually summed.
///
/// A coarse octave keeps the same weight when a later call adds finer ones.
/// The octave seeds match [`fbm`]. `gain >= 1` falls back to the present-octave sum.
pub fn fbm_fixed_norm(
    seed: u64,
    p: [f64; 3],
    base_frequency: f64,
    octaves: u32,
    lacunarity: f64,
    gain: f64,
) -> f64 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut freq = base_frequency;
    let mut present = 0.0;
    for octave in 0..octaves {
        let s = splitmix64(seed ^ (octave as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        sum += amp * noise_on_sphere(s, p, freq);
        present += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    let fixed = if gain < 1.0 {
        (1.0 - gain).recip()
    } else {
        present
    };
    if fixed == 0.0 {
        0.0
    } else {
        sum / fixed
    }
}

/// Ridged multifractal: octave-weighted mean of `(1 − |noise|)²`, in about `[0, 1]`.
pub fn ridged(
    seed: u64,
    p: [f64; 3],
    base_frequency: f64,
    octaves: u32,
    lacunarity: f64,
    gain: f64,
) -> f64 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut freq = base_frequency;
    let mut norm = 0.0;
    for octave in 0..octaves {
        let s = splitmix64(seed ^ (octave as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
        let n = noise_on_sphere(s, p, freq).clamp(-1.0, 1.0);
        let r = 1.0 - n.abs();
        sum += amp * r * r;
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    if norm == 0.0 {
        0.0
    } else {
        sum / norm
    }
}

/// Displace `p` by three noise components and renormalize to a unit vector.
pub fn warp(seed: u64, p: [f64; 3], amplitude: f64, frequency: f64) -> [f64; 3] {
    let sx = seed;
    let sy = splitmix64(seed ^ 0xA5A5_A5A5_A5A5_A5A5);
    let sz = splitmix64(seed ^ 0x5A5A_5A5A_5A5A_5A5A);
    let dx = noise_on_sphere(sx, p, frequency);
    let dy = noise_on_sphere(sy, p, frequency);
    let dz = noise_on_sphere(sz, p, frequency);
    crate::grid::normalize([
        p[0] + amplitude * dx,
        p[1] + amplitude * dy,
        p[2] + amplitude * dz,
    ])
}
