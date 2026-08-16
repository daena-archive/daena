# ADR 0050: Physical generator 13 — motion-scaled orogeny, landmass variety, poles, coastal erosion

- Status: Accepted
- Date: 2026-08-16
- Scope: `crates/daena-physical-spike` tectonic cause chain and terrain
  evolution. Source format stays `physical-world-v2`. Generator version is
  `13`. Evolution derivation is `4`.

## Context

Generator `12` (ADR 0035) already stores per-plate Euler speeds and
per-boundary relative motion, and it already splits/merges plates. Collision,
trench, and rift kernels ignored that speed. Continent growth *penalized*
plate-crossing, so land avoided junctions. Cratons were clamped off the poles.
Balanced shatter/merge kept plate count fixed, so huge landmasses stayed rare.
Stream-power skipped the sea and had no coastal or mouth deposition term.

## Decisions

1. Scale collision, trench, inland/oceanic arc, rift, and ridge amplitudes
   (and mildly their widths) by `relative_speed / 600_000`, clamped
   `0.40`–`2.20`. Continent-continent collision is biased toward the slower
   plate, with a foreland basin on the faster side. Trenches stay on the
   oceanic (downgoing) side of mixed contacts and on the faster plate of
   ocean-ocean contacts.
2. Continent growth subtracts a junction attraction at triple junctions, with
   a stronger bonus on convergent contacts and a weaker bonus on two-plate
   convergent edges. Crossing a plate that is not the craton's home plate
   remains a cost, so quiet interiors do not smear. Cratons in the same group
   share one home plate. Craton seeds stay clamped off the poles, as in
   generator `12`.
3. After balanced reshape, extra adjacent-plate merges may reduce plate count
   (not below 4). Layout weights are Mega `24` / Giant+scraps `18` / Dual
   `28` / Scattered `30`. Mega crust target is `520_000` ppm. Giant+scraps
   keeps one dominant group plus smaller continents.
4. Evolution incision is stronger within ~350 km of the ocean after the land
   outline has been stable for half the steps, scaled by coastal exposure
   (open-ocean neighbor fraction at the nearest shore). High-accumulation
   mouths cut more; extra coastal incision cannot convert land to ocean.
   Low-slope, high-accumulation land then receives bounded deposition.
   Ocean cells stay skipped for hillslope diffusion.
   Evolution derivation is `4`. Hydrology derivation is `4`. The locked
   wall-time gate is `10 s`.

## Consequences

Accepted v12 worlds are not reinterpreted. New generation writes v13 sources.
Atlas remains a derived consumer of the accepted field. Tests check
determinism (same seed, same bytes twice) and structural budgets; they do not
lock SHA-256 goldens.
