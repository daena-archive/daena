//! Hash-addressed 3D gradient noise. No permutation table.

use sha2::{Digest, Sha256};

/// Mean of `1 - 2·|n|` for [`perlin`], measured over 10⁶ samples.
///
/// Octave 0 of domain `"terrain"` seeded by `b"ridged-constant"`. The same
/// lattice is checked by `ridged_noise_mean_constant_is_reproduced`.
pub const RIDGED_NOISE_MEAN: f64 = 0.690_117_687_109_039_8;

const PERLIN_SCALE: f64 = std::f64::consts::FRAC_1_SQRT_2;

const GRADIENTS: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

pub(super) struct Octave {
    seed: u64,
    offset: [f64; 3],
    rot: [[f64; 3]; 3],
}

/// `u64` from SHA-256 of the terrain domain prefix and the identity bytes.
pub(super) fn terrain_seed(identity: &[u8]) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(b"daena-atlas-terrain-v1\0");
    hasher.update(identity);
    let digest = hasher.finalize();
    u64::from_le_bytes(digest[..8].try_into().expect("sha256 prefix"))
}

/// Domain seed. The hash takes the domain name, so later phases stay independent.
pub(super) fn domain_seed(base: u64, domain: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(b"daena-atlas-terrain-domain\0");
    hasher.update(base.to_le_bytes());
    hasher.update(domain.as_bytes());
    let digest = hasher.finalize();
    u64::from_le_bytes(digest[..8].try_into().expect("sha256 prefix"))
}

pub(super) fn build_octaves(domain: u64, count: usize) -> Vec<Octave> {
    (0..count)
        .map(|index| octave_from_seed(octave_seed(domain, index as u32)))
        .collect()
}

fn octave_seed(domain: u64, index: u32) -> u64 {
    splitmix64(domain.wrapping_add((index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

fn octave_from_seed(seed: u64) -> Octave {
    let s1 = splitmix64(seed);
    let s2 = splitmix64(s1);
    let s3 = splitmix64(s2);
    let s4 = splitmix64(s3);
    let u1 = unit_open(s2);
    let u2 = unit_open(s3);
    let u3 = unit_open(s4);
    let a = (1.0 - u1).sqrt();
    let b = u1.sqrt();
    let t2 = std::f64::consts::TAU * u2;
    let t3 = std::f64::consts::TAU * u3;
    let quat = [a * t2.sin(), a * t2.cos(), b * t3.sin(), b * t3.cos()];
    Octave {
        seed: s1,
        offset: [
            unit_open(splitmix64(s4.wrapping_add(1))) * 64.0,
            unit_open(splitmix64(s4.wrapping_add(2))) * 64.0,
            unit_open(splitmix64(s4.wrapping_add(3))) * 64.0,
        ],
        rot: quat_to_mat(quat),
    }
}

/// Noise at `rotate(p) · freq + offset`, in `[-1, 1]`.
#[inline]
pub(super) fn sample_octave(octave: &Octave, direction: [f64; 3], frequency: f64) -> f64 {
    let rotated = mul_mat(octave.rot, direction);
    perlin(
        octave.seed,
        rotated[0] * frequency + octave.offset[0],
        rotated[1] * frequency + octave.offset[1],
        rotated[2] * frequency + octave.offset[2],
    )
}

#[inline]
pub(super) fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn unit_open(hash: u64) -> f64 {
    (hash >> 11) as f64 / ((1u64 << 53) as f64)
}

fn quat_to_mat(q: [f64; 4]) -> [[f64; 3]; 3] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
        ],
        [
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
        ],
        [
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ]
}

fn mul_mat(rot: [[f64; 3]; 3], p: [f64; 3]) -> [f64; 3] {
    [
        rot[0][0] * p[0] + rot[0][1] * p[1] + rot[0][2] * p[2],
        rot[1][0] * p[0] + rot[1][1] * p[1] + rot[1][2] * p[2],
        rot[2][0] * p[0] + rot[2][1] * p[1] + rot[2][2] * p[2],
    ]
}

#[inline]
fn lattice_hash(seed: u64, x: i32, y: i32, z: i32) -> u64 {
    let mut h = seed;
    h = splitmix64(h ^ (x as u32 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    h = splitmix64(h ^ (y as u32 as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
    splitmix64(h ^ (z as u32 as u64).wrapping_mul(0xD1B5_4A32_D192_ED03))
}

#[inline]
fn grad_dot(hash: u64, x: f64, y: f64, z: f64) -> f64 {
    let g = GRADIENTS[(hash % 12) as usize];
    g[0] * x + g[1] * y + g[2] * z
}

#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Improved Perlin noise, scaled into `[-1, 1]`.
#[inline]
pub(super) fn perlin(seed: u64, x: f64, y: f64, z: f64) -> f64 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let z0 = z.floor() as i32;
    let xf = x - f64::from(x0);
    let yf = y - f64::from(y0);
    let zf = z - f64::from(z0);
    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);
    let n000 = grad_dot(lattice_hash(seed, x0, y0, z0), xf, yf, zf);
    let n100 = grad_dot(lattice_hash(seed, x0 + 1, y0, z0), xf - 1.0, yf, zf);
    let n010 = grad_dot(lattice_hash(seed, x0, y0 + 1, z0), xf, yf - 1.0, zf);
    let n110 = grad_dot(
        lattice_hash(seed, x0 + 1, y0 + 1, z0),
        xf - 1.0,
        yf - 1.0,
        zf,
    );
    let n001 = grad_dot(lattice_hash(seed, x0, y0, z0 + 1), xf, yf, zf - 1.0);
    let n101 = grad_dot(
        lattice_hash(seed, x0 + 1, y0, z0 + 1),
        xf - 1.0,
        yf,
        zf - 1.0,
    );
    let n011 = grad_dot(
        lattice_hash(seed, x0, y0 + 1, z0 + 1),
        xf,
        yf - 1.0,
        zf - 1.0,
    );
    let n111 = grad_dot(
        lattice_hash(seed, x0 + 1, y0 + 1, z0 + 1),
        xf - 1.0,
        yf - 1.0,
        zf - 1.0,
    );
    let x00 = lerp(n000, n100, u);
    let x10 = lerp(n010, n110, u);
    let x01 = lerp(n001, n101, u);
    let x11 = lerp(n011, n111, u);
    let y0 = lerp(x00, x10, v);
    let y1 = lerp(x01, x11, v);
    (lerp(y0, y1, w) * PERLIN_SCALE).clamp(-1.0, 1.0)
}

/// Mean of `1 - 2·|n|` and the max `|n|` over a 100³ lattice (10⁶ samples).
#[cfg(test)]
pub(super) fn measure_ridged_mean() -> (f64, f64) {
    let seed = octave_seed(domain_seed(terrain_seed(b"ridged-constant"), "terrain"), 0);
    let mut acc = 0.0;
    let mut max_abs: f64 = 0.0;
    for z in 0..100u32 {
        for y in 0..100u32 {
            for x in 0..100u32 {
                let value = perlin(
                    seed,
                    f64::from(x) * 0.17 + 0.13,
                    f64::from(y) * 0.17 + 0.29,
                    f64::from(z) * 0.13 + 0.41,
                );
                max_abs = max_abs.max(value.abs());
                acc += 1.0 - 2.0 * value.abs();
            }
        }
    }
    (acc / 1_000_000.0, max_abs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ridged_noise_mean_constant_is_reproduced() {
        let (mean, max_abs) = measure_ridged_mean();
        assert!(
            max_abs <= 1.0 + 1e-9 && (mean - RIDGED_NOISE_MEAN).abs() <= 0.01,
            "measured mean {mean} max {max_abs} constant {RIDGED_NOISE_MEAN}"
        );
    }

    #[test]
    fn domain_and_identity_change_the_seed() {
        assert_ne!(terrain_seed(b"a"), terrain_seed(b"b"));
        assert_ne!(domain_seed(1, "terrain"), domain_seed(1, "water"));
        assert_ne!(octave_seed(7, 0), octave_seed(7, 1));
    }
}
