export type PlanetaryPreset = "earth-like" | "low-tilt" | "high-tilt" | "slow-rotating" | "close-orbit" | "custom";

export interface PlanetaryConfiguration {
  version: 1;
  preset: PlanetaryPreset;
  starLuminosityPpm: number;
  starMassPpm: number;
  semiMajorAxisMilliAu: number;
  eccentricityPpm: number;
  axialTiltMilliDeg: number;
  rotationPeriodSeconds: number;
  retainedHeatCentiC: number;
  bondAlbedoPpm: number;
  meanDensityKgM3: number;
  radiusMetres: number;
}

const AU_METRES = 149_597_870_700;
const GRAVITATIONAL_CONSTANT = 6.6743e-11;
const SOLAR_MASS_KG = 1.988416e30;
const EARTH_GRAVITY_MS2 = 9.80665;
export const EARTH_RADIUS_METRES = 6_371_000;

export const PLANETARY_LIMITS = {
  starLuminosityPpm: [10_000, 100_000_000],
  starMassPpm: [80_000, 8_000_000],
  semiMajorAxisMilliAu: [50_000, 50_000_000],
  eccentricityPpm: [0, 800_000],
  axialTiltMilliDeg: [0, 90_000],
  rotationPeriodSeconds: [3_600, 7_776_000],
  retainedHeatCentiC: [-5_000, 5_000],
  bondAlbedoPpm: [50_000, 800_000],
  meanDensityKgM3: [1_000, 12_000],
  radiusMetres: [1, Number.MAX_SAFE_INTEGER],
} as const;

export function earthLikePlanetary(): PlanetaryConfiguration {
  return {
    version: 1,
    preset: "earth-like",
    starLuminosityPpm: 1_000_000,
    starMassPpm: 1_000_000,
    semiMajorAxisMilliAu: 1_000_000,
    eccentricityPpm: 16_710,
    axialTiltMilliDeg: 23_440,
    rotationPeriodSeconds: 86_400,
    retainedHeatCentiC: 1_400,
    bondAlbedoPpm: 306_000,
    meanDensityKgM3: 5_514,
    radiusMetres: EARTH_RADIUS_METRES,
  };
}

export function planetaryFromPreset(preset: PlanetaryPreset): PlanetaryConfiguration {
  const configuration = earthLikePlanetary();
  configuration.preset = preset;
  if (preset === "low-tilt") configuration.axialTiltMilliDeg = 5_000;
  if (preset === "high-tilt") configuration.axialTiltMilliDeg = 40_000;
  if (preset === "slow-rotating") configuration.rotationPeriodSeconds = 259_200;
  if (preset === "close-orbit") configuration.semiMajorAxisMilliAu = 700_000;
  return configuration;
}

export function markPlanetaryCustom(configuration: PlanetaryConfiguration): PlanetaryConfiguration {
  return { ...configuration, preset: "custom" };
}

export function validatePlanetary(configuration: PlanetaryConfiguration): string | null {
  if (configuration.version !== 1) return "Unsupported planetary configuration version.";
  for (const [key, [min, max]] of Object.entries(PLANETARY_LIMITS) as [
    keyof typeof PLANETARY_LIMITS,
    readonly [number, number],
  ][]) {
    const value = configuration[key];
    if (!Number.isFinite(value) || value < min || value > max) {
      return "Planetary configuration is outside the supported range.";
    }
  }
  return null;
}

function finiteOrNull(value: number): number | null {
  return Number.isFinite(value) ? value : null;
}

export function orbitalPeriodSeconds(configuration: PlanetaryConfiguration): number | null {
  if (validatePlanetary(configuration)) return null;
  const massKg = (SOLAR_MASS_KG * configuration.starMassPpm) / 1_000_000;
  const axisMetres = (AU_METRES * configuration.semiMajorAxisMilliAu) / 1_000_000;
  return finiteOrNull(2 * Math.PI * Math.sqrt(axisMetres ** 3 / (GRAVITATIONAL_CONSTANT * massKg)));
}

export function insolationPpm(configuration: PlanetaryConfiguration): number | null {
  if (validatePlanetary(configuration)) return null;
  const au = configuration.semiMajorAxisMilliAu / 1_000_000;
  return finiteOrNull(configuration.starLuminosityPpm / (au * au));
}

export function surfaceGravityMilliG(configuration: PlanetaryConfiguration): number | null {
  if (validatePlanetary(configuration)) return null;
  const gravity =
    (((4 / 3) * Math.PI * GRAVITATIONAL_CONSTANT * configuration.meanDensityKgM3 * configuration.radiusMetres) /
      EARTH_GRAVITY_MS2) *
    1_000;
  return finiteOrNull(gravity);
}

export function withOrbitalPeriodSeconds(
  configuration: PlanetaryConfiguration,
  seconds: number,
): PlanetaryConfiguration | null {
  if (!Number.isFinite(seconds) || seconds <= 0) return null;
  const massKg = (SOLAR_MASS_KG * configuration.starMassPpm) / 1_000_000;
  const axisMetres = ((seconds / (2 * Math.PI)) ** 2 * GRAVITATIONAL_CONSTANT * massKg) ** (1 / 3);
  const semiMajorAxisMilliAu = Math.round((axisMetres / AU_METRES) * 1_000_000);
  const next = markPlanetaryCustom({ ...configuration, semiMajorAxisMilliAu });
  return validatePlanetary(next) ? null : next;
}
