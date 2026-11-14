//! Water map for the realism review: ocean, lakes, rivers and ice.
//!
//! Usage: realism_water <preset> <seed> [n] <out.png>
use daena_worldgen::{generate, write_png, NoProgress, Preset};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let preset = Preset::parse(&args[1]).expect("preset");
    let seed: u64 = args[2].parse().expect("seed");
    let n: u32 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(512);
    let out = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| "water.png".to_string());
    let mut settings = preset.settings();
    settings.seed = seed;
    settings.resolution = n;
    let world = generate(&settings, &mut NoProgress).expect("generate");
    let view = world.equirect_view(2048, 1024).expect("equirect");
    let mut rgb = Vec::with_capacity((view.width * view.height * 3) as usize);
    for y in 0..view.height {
        // `equirect_view` rows run south to north; PNG rows run north to south.
        let src_y = view.height - 1 - y;
        for x in 0..view.width {
            let i = (src_y * view.width + x) as usize;
            let ice = view.ice_cover.get(i).copied().unwrap_or(0);
            let pixel = if ice > 0 {
                [236, 242, 248]
            } else if view.ocean.get(i).copied().unwrap_or(0) == 1 {
                [18, 48, 92]
            } else if view.lake_id.get(i).copied().unwrap_or(u32::MAX) != u32::MAX {
                [44, 112, 184]
            } else if view.discharge_m3s.get(i).copied().unwrap_or(0.0) >= 50.0 {
                [70, 148, 214]
            } else {
                [108, 116, 102]
            };
            rgb.extend_from_slice(&pixel);
        }
    }
    write_png(std::path::Path::new(&out), view.width, view.height, &rgb).expect("png");
}
