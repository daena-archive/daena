use crate::noise::{derive_seed, fbm, fbm_fixed_norm, noise_on_sphere, splitmix64, SeedDomain};

#[test]
fn same_seed_same_value_and_domains_differ() {
    let p = [0.3_f64, -0.4, 0.866_025_403_78];
    let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    let p = [p[0] / len, p[1] / len, p[2] / len];
    let plates = derive_seed(7, SeedDomain::Plates);
    let continents = derive_seed(7, SeedDomain::Continents);
    let a = fbm(plates, p, 2.0, 5, 2.0, 0.5);
    let b = fbm(plates, p, 2.0, 5, 2.0, 0.5);
    let c = fbm(continents, p, 2.0, 5, 2.0, 0.5);
    assert_eq!(a.to_bits(), b.to_bits());
    assert_ne!(a.to_bits(), c.to_bits());
    assert!(a.is_finite() && c.is_finite());
}

#[test]
fn fixed_norm_does_not_rescale_the_coarse_octave() {
    let p = [0.3_f64, -0.4, 0.866_025_403_78];
    let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    let p = [p[0] / len, p[1] / len, p[2] / len];
    let seed = derive_seed(7, SeedDomain::UpliftDetail);
    let freq = 32.0 / std::f64::consts::PI;
    let octave = |index: u64, frequency: f64| {
        let mixed = splitmix64(seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        noise_on_sphere(mixed, p, frequency)
    };
    let coarse = fbm_fixed_norm(seed, p, freq, 1, 2.0, 0.5);
    let fine = fbm_fixed_norm(seed, p, freq, 2, 2.0, 0.5);
    let n0 = octave(0, freq);
    let n1 = octave(1, freq * 2.0);
    assert!((coarse - n0 / 2.0).abs() < 1.0e-12);
    assert!((fine - (n0 + 0.5 * n1) / 2.0).abs() < 1.0e-12);
}
