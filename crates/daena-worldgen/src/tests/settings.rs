use crate::{
    generate, ContinentLayout, ErosionParams, NoProgress, Schedule, ScheduleStep, WorldError,
    WorldSettings,
};

fn rejected(settings: WorldSettings) {
    let err = generate(&settings, &mut NoProgress).unwrap_err();
    assert!(matches!(err, WorldError::InvalidSettings(_)), "{err}");
}

#[test]
fn resolution_range() {
    rejected(WorldSettings {
        resolution: 16,
        ..WorldSettings::default()
    });
    rejected(WorldSettings {
        resolution: 48,
        ..WorldSettings::default()
    });
    rejected(WorldSettings {
        resolution: 4096,
        ..WorldSettings::default()
    });
    assert!(WorldSettings {
        resolution: 32,
        ..WorldSettings::default()
    }
    .validate()
    .is_ok());
    assert!(WorldSettings {
        resolution: 2048,
        ..WorldSettings::default()
    }
    .validate()
    .is_ok());
}

#[test]
fn land_fraction_range() {
    for value in [0.019, 0.91, f64::NAN] {
        rejected(WorldSettings {
            land_fraction: value,
            ..WorldSettings::default()
        });
    }
    for value in [0.02, 0.9] {
        let settings = WorldSettings {
            land_fraction: value,
            ..WorldSettings::default()
        };
        assert!(settings.validate().is_ok(), "{value}");
    }
}

#[test]
fn plate_count_range() {
    for value in [0, 1, 49, 255] {
        rejected(WorldSettings {
            plate_count: value,
            ..WorldSettings::default()
        });
    }
    for value in [2, 48] {
        let settings = WorldSettings {
            plate_count: value,
            ..WorldSettings::default()
        };
        assert!(settings.validate().is_ok(), "{value}");
    }
}

#[test]
fn tectonic_activity_range() {
    for value in [0.19, 2.01, f64::INFINITY] {
        rejected(WorldSettings {
            tectonic_activity: value,
            ..WorldSettings::default()
        });
    }
    for value in [0.2, 2.0] {
        let settings = WorldSettings {
            tectonic_activity: value,
            ..WorldSettings::default()
        };
        assert!(settings.validate().is_ok(), "{value}");
    }
}

#[test]
fn radius_range() {
    for value in [9.9e5, 2.01e7, -1.0, f64::NAN] {
        rejected(WorldSettings {
            radius_m: value,
            ..WorldSettings::default()
        });
    }
    for value in [1.0e6, 2.0e7] {
        let settings = WorldSettings {
            radius_m: value,
            ..WorldSettings::default()
        };
        assert!(settings.validate().is_ok(), "{value}");
    }
}

#[test]
fn schedule_range() {
    let mut zero_iters = WorldSettings::default();
    zero_iters.schedule.steps[0].iterations = 0;
    rejected(zero_iters);
    let mut zero_dt = WorldSettings::default();
    zero_dt.schedule.steps[0].dt_yr = 0.0;
    rejected(zero_dt);
    rejected(WorldSettings {
        schedule: Schedule { steps: Vec::new() },
        ..WorldSettings::default()
    });
    rejected(WorldSettings {
        schedule: Schedule {
            steps: vec![ScheduleStep {
                max_n: 16,
                iterations: 2,
                dt_yr: 1.0,
            }],
        },
        ..WorldSettings::default()
    });
    assert!(WorldSettings::default().validate().is_ok());
}

#[test]
fn erosion_range() {
    let bad = [
        ErosionParams {
            m: 0.0,
            ..ErosionParams::default()
        },
        ErosionParams {
            k: -1.0,
            ..ErosionParams::default()
        },
        ErosionParams {
            s_c: f64::NAN,
            ..ErosionParams::default()
        },
        ErosionParams {
            kappa: 0.0,
            ..ErosionParams::default()
        },
        ErosionParams {
            max_elevation_m: f64::NEG_INFINITY,
            ..ErosionParams::default()
        },
    ];
    for params in bad {
        rejected(WorldSettings {
            erosion: params,
            ..WorldSettings::default()
        });
    }
    let _ = ContinentLayout::default();
    assert!(WorldSettings::default().validate().is_ok());
}
