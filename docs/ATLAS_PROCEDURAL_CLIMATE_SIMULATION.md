# Atlas Procedural Climate Simulation

## Purpose

Atlas should generate climate as the result of interacting physical processes rather than independently generating temperature, wind, rainfall, humidity, and biomes as unrelated procedural fields.

The objective is **believability, continuity, and causality**, not numerical accuracy.

A generated world should produce recognizable consequences such as:

- warm equatorial regions and colder polar regions
- seasonal temperature changes
- heat transported away from strongly heated regions
- prevailing winds influenced by planetary rotation
- moisture transported from oceans toward land
- increased precipitation on windward mountain slopes
- rain shadows behind mountain ranges
- deserts associated with persistent dry circulation
- wet tropical regions where heat and moisture coincide
- storm-prone regions where atmospheric conditions favor instability
- coherent climate zones rather than noisy biome patches

The simulation should therefore operate on persistent environmental state and repeatedly update that state until a stable or quasi-stable climate emerges.

---

# 1. Simulation Philosophy

The simulation should follow this causal chain:

```text
Planetary parameters
        ↓
Solar radiation
        ↓
Initial thermal state
        ↓
Pressure distribution
        ↓
Atmospheric circulation
        ↓
Heat + moisture transport
        ↓
Condensation / precipitation
        ↓
Surface and atmospheric feedback
        ↓
Updated temperature / pressure / circulation
        ↓
Repeat
        ↓
Climate statistics
        ↓
Climate classification / biomes
```

The important property is that each iteration operates on the **results of previous iterations**.

Temperature should not be regenerated after wind simulation. Rainfall should not be assigned independently from humidity. Wind should not be a random vector field.

Instead:

> heat creates pressure differences, pressure creates circulation, circulation transports heat and moisture, moisture produces precipitation, and these processes modify the next state.

This is a simplified numerical climate model.

It is not intended to reproduce weather on an Earth-like planet with scientific accuracy.

---

# 2. Spatial Representation

The world is divided into cells.

Each cell should maintain a climate state containing at minimum:

```text
temperature
pressure
wind vector
humidity
precipitation
surface water
```

Useful additional state:

```text
cloud water
snow / ice
ocean temperature
soil moisture
solar energy
elevation
albedo
```

Most variables are continuous scalar fields.

Wind is a two-dimensional vector field.

The simulation should operate on neighboring cells so that fields can exchange heat and moisture.

---

# 3. Planetary Parameters

The climate model should derive its forcing from a small number of planetary parameters.

## Essential parameters

### Stellar luminosity

Controls the total energy available from the star.

### Orbital distance

Controls incident stellar radiation.

For an approximate inverse-square relationship:

\[
S(r) = S_0 \left(\frac{r_0}{r}\right)^2
\]

where:

- \(S\) = stellar irradiance
- \(S_0\) = reference irradiance
- \(r\) = current orbital distance
- \(r_0\) = reference distance

### Axial tilt

Controls seasonal variation in solar incidence.

### Rotation period

Controls Coriolis strength and therefore atmospheric circulation.

### Atmospheric heat retention

Represents the combined greenhouse and atmospheric insulation effect.

This should be an abstract parameter rather than attempting to model every atmospheric constituent.

### Atmospheric density / pressure

Controls the effectiveness of atmospheric heat and moisture transport.

### Surface albedo

Controls how much incoming radiation is reflected.

### Terrain elevation

Controls:

- temperature
- atmospheric pressure
- orographic precipitation
- circulation
- snow and ice

### Ocean / land distribution

Oceans should act as large thermal and moisture reservoirs.

---

# 4. Initial Solar Radiation

For each cell, calculate the average or instantaneous solar energy received by the surface.

A simplified latitude-dependent approximation is sufficient:

\[
I \propto S \cdot \cos(\theta)
\]

where \(\theta\) is the angle between the incoming solar ray and the surface normal.

For annual-average initialization, a simpler latitude function can be used:

\[
I(\phi)=I_0 f(\phi,\epsilon)
\]

where:

- \(\phi\) = latitude
- \(\epsilon\) = axial tilt

For seasonal simulation, calculate the solar declination from the orbital position and tilt.

The precise astronomical equation is not essential. What matters is producing:

- strongest average heating near the equator
- seasonal migration of the strongest heating
- stronger seasonal variation at high latitudes
- little seasonal variation near the equator

---

# 5. Initial Temperature Map

The initial temperature map should be an **approximation of radiative equilibrium**, not a random latitude gradient.

A simplified radiative equilibrium relationship is:

\[
T_{eq} =
\left(
\frac{S(1-\alpha)}
{\epsilon\sigma}
\right)^{1/4}
\]

where:

- \(T_{eq}\) = equilibrium temperature
- \(S\) = incoming solar radiation
- \(\alpha\) = albedo
- \(\epsilon\) = effective emissivity
- \(\sigma\) = Stefan-Boltzmann constant

This equation should not be treated as the final climate temperature.

It provides a physically useful starting point.

## Recommended abstraction

Instead of exposing emissivity directly, define an abstract atmospheric retention parameter:

\[
T_{base}=F(T_{eq}, G)
\]

where \(G\) controls atmospheric heat retention.

The exact function can be tuned to produce desirable world ranges.

---

# 6. Surface Effects

Apply modifiers to the initial temperature.

## Elevation

Temperature generally decreases with altitude.

A simple lapse approximation:

\[
T_{alt}=T-\Gamma h
\]

where:

- \(h\) = elevation
- \(\Gamma\) = configurable lapse coefficient

Do not require the coefficient to represent Earth's exact atmospheric lapse rate.

The important behavior is:

> mountains and high plateaus should be colder than nearby lowlands.

## Water

Water should change how quickly temperature responds to solar forcing.

For a simplified model:

\[
C_{cell}\frac{\partial T}{\partial t}=Q_{net}
\]

where \(C_{cell}\) is effective thermal capacity.

Use a substantially larger thermal capacity for ocean cells.

Consequences:

- oceans heat slowly
- oceans cool slowly
- coastlines have moderated temperatures
- continental interiors experience larger seasonal extremes

This distinction is highly important for believable climate.

## Albedo

Different surfaces reflect different fractions of incoming radiation.

\[
Q_{absorbed}=Q_{solar}(1-\alpha)
\]

At minimum, distinguish:

```text
ocean
vegetated land
bare land
desert
snow
ice
```

Exact values can be tuned.

---

# 7. Temperature Evolution

The basic temperature equation should contain four major terms:

\[
C\frac{\partial T}{\partial t}
=

Q_{solar}
-

Q_{outgoing} +
Q_{transport} +
Q_{latent}
\]

where:

- \(C\) = effective heat capacity
- \(Q_{solar}\) = absorbed solar energy
- \(Q_{outgoing}\) = emitted thermal radiation
- \(Q_{transport}\) = heat transported into/out of the cell
- \(Q_{latent}\) = latent heat effects

A simplified outgoing radiation model is:

\[
Q_{outgoing}=A+B T
\]

This is a common approximation in energy-balance models because it avoids solving the complete radiative-transfer problem.

A more physically shaped alternative is:

\[
Q_{outgoing}=\epsilon\sigma T^4
\]

Either is acceptable.

For procedural generation, the linearized form is generally easier to control.

---

# 8. Heat Transport

A simplified climate model can transport heat down temperature gradients.

The classical diffusive approximation is:

\[
\mathbf{H}=-K_T\nabla T
\]

where:

- \(\mathbf{H}\) = heat flux
- \(K_T\) = thermal diffusivity
- \(\nabla T\) = temperature gradient

The divergence of this flux changes local temperature:

\[
Q_{transport}=-\nabla\cdot\mathbf{H}
\]

This is the fundamental mechanism used by simplified energy-balance models to represent unresolved atmospheric and oceanic heat transport.

However, Atlas should eventually combine diffusion with explicit wind transport.

### Diffusion handles

- unresolved large-scale circulation
- oceanic redistribution
- smoothing
- numerical stability

### Wind advection handles

- directional heat movement
- prevailing circulation
- continental effects
- realistic asymmetric climate patterns

Use both.

---

# 9. Pressure

A complete atmospheric pressure model is unnecessary.

The simulation needs pressure primarily because pressure gradients create atmospheric circulation.

Establish a broad baseline pressure field using latitude and planetary circulation.

A simplified conceptual form is:

\[
P_{base}=F(\phi,\Omega,T_{global})
\]

Then introduce local thermal perturbations:

\[
P=P_{base}+P_{thermal}
\]

with:

\[
P_{thermal}=-K_P(T-T_{regional})
\]

This creates lower pressure where air is anomalously warm and higher pressure where it is anomalously cold.

Pressure should also decrease with elevation.

A simplified exponential relationship is:

\[
P(h)=P_0e^{-h/H}
\]

where \(H\) is an adjustable atmospheric scale height.

For Atlas, this only needs to preserve the correct qualitative behavior.

---

# 10. Pressure Gradient

Calculate the horizontal pressure gradient:

\[
\mathbf{G}_P=-\nabla P
\]

The resulting vector indicates the direction of pressure-gradient acceleration.

Do not simply set wind direction equal to this vector.

Rotation must modify it.

---

# 11. Planetary Rotation and Coriolis Effect

Rotation is essential.

The Coriolis parameter is:

\[
f=2\Omega\sin(\phi)
\]

where:

- \(\Omega\) = angular rotation rate
- \(\phi\) = latitude

This provides three important behaviors:

1. Coriolis force reverses direction between hemispheres.
2. Coriolis force approaches zero near the equator.
3. Faster rotation produces stronger atmospheric deflection.

This allows rotation speed to become a meaningful planetary parameter.

---

# 12. Wind Model

A full fluid-dynamics solver is unnecessary.

Use a parameterized momentum equation:

\[
\frac{d\mathbf{V}}{dt}
=

-K_P\nabla P +
K_C\mathbf{C}
-

K_D\mathbf{V}
\]

where:

- \(\mathbf{V}\) = wind velocity
- \(-\nabla P\) = pressure-gradient acceleration
- \(\mathbf{C}\) = Coriolis acceleration
- \(K_D\) = atmospheric drag

For horizontal wind:

\[
\mathbf{C}\approx
f
\begin{bmatrix}
-v\\
u
\end{bmatrix}
\]

for suitable coordinate orientation.

The important result is that winds should generally be **deflected relative to pressure gradients**, rather than flowing directly from high pressure toward low pressure.

Geostrophic approximations similarly produce wind approximately perpendicular to pressure gradients.

---

# 13. Surface Friction

Wind should be weakened near the surface.

A simplified drag term:

\[
\mathbf{V}_{drag}=-K_D\mathbf{V}
\]

Make \(K_D\) dependent on surface type:

```text
ocean       low/moderate
grassland   moderate
forest      high
mountains   high
urban       irrelevant for planetary generation
```

Exact values do not matter nearly as much as having different roughness.

---

# 14. Wind Advection

Wind should transport properties between cells.

For any transported scalar \(X\):

\[
\frac{\partial X}{\partial t} +
\mathbf{V}\cdot\nabla X
=

S_X
\]

where:

- \(X\) = temperature, humidity, etc.
- \(\mathbf{V}\) = wind
- \(S_X\) = local source/sink

For Atlas, a first-order upwind advection scheme is sufficient.

Numerical accuracy is less important than:

- conservation
- stability
- no obvious artifacts
- smooth transport
- respecting coastlines and terrain

---

# 15. Atmospheric Moisture

Track atmospheric moisture as a scalar:

\[
q = \text{water vapor content}
\]

Ocean and wet land are moisture sources.

Dry land is not an automatic moisture source.

Humidity should move according to wind:

\[
\frac{\partial q}{\partial t} +
\mathbf{V}\cdot\nabla q
=

E-C
\]

where:

- \(E\) = evaporation
- \(C\) = condensation

---

# 16. Evaporation

Evaporation should depend primarily on:

- water availability
- surface temperature
- atmospheric humidity
- wind speed

A simplified formulation:

\[
E=
K_E
W
|\mathbf{V}|
(e_s(T)-e)
\]

where:

- \(W\) = water availability
- \(e_s(T)\) = saturation vapor pressure
- \(e\) = current vapor pressure
- \(K_E\) = tunable coefficient

A simpler procedural approximation can use normalized humidity instead:

\[
E=
K_E W |\mathbf{V}|(1-RH)
\]

where \(RH\) is relative humidity.

The important behavior is:

> warm, dry, windy surfaces evaporate more water.

---

# 17. Saturation

Air cannot indefinitely accumulate water vapor.

Saturation vapor pressure increases rapidly with temperature.

The Clausius–Clapeyron relation is:

\[
\frac{d e_s}{dT}
=

\frac{L e_s}{R_vT^2}
\]

With approximately constant latent heat:

\[
e_s(T)
=

e_0
\exp
\left[
\frac{L}{R_v}
\left(
\frac{1}{T_0}-\frac{1}{T}
\right)
\right]
\]

This simplified formulation is commonly used in climate and idealized atmospheric models.

For Atlas, a lookup table is preferable to repeatedly evaluating the exponential.

---

# 18. Relative Humidity

Calculate:

\[
RH=\frac{e}{e_s(T)}
\]

When:

\[
RH>1
\]

the cell contains more vapor than can remain suspended.

The excess becomes condensate:

\[
C=K_C\max(0,q-q_{sat})
\]

Then:

\[
q\leftarrow q-C
\]

This provides a natural rainfall source.

The idealized atmospheric models commonly enforce the same basic constraint: relative humidity cannot remain above saturation indefinitely.

---

# 19. Atmospheric Uplift

Condensation is strongly associated with air being cooled as it rises.

Atlas does not need to model vertical atmospheric columns.

Instead, define simplified uplift factors from:

- convection
- terrain
- convergence

## Orographic uplift

Calculate the terrain gradient:

\[
\nabla h
\]

Then:

\[
U_{oro}=
\max(0,\mathbf{V}\cdot\nabla h)
\]

A positive value means wind is moving uphill.

This formulation is directly related to simplified orographic precipitation models, where vertical motion is parameterized from the dot product of wind and terrain gradient.

---

# 20. Cooling From Uplift

Instead of explicitly simulating atmospheric height, use uplift to increase the probability/rate of cooling and condensation.

For example:

\[
T_{effective}
=

T-K_UU_{oro}
\]

Then calculate saturation from the effective temperature.

This produces:

```text
moist wind
   ↓
mountain slope
   ↓
uplift
   ↓
cooling
   ↓
lower saturation capacity
   ↓
condensation
   ↓
rain
```

---

# 21. Precipitation

A simple precipitation formulation is:

\[
R=
K_R\max(0,q-q_{sat})
\]

with an additional uplift multiplier:

\[
R=
K_R
\max(0,q-q_{sat})
(1+K_UU)
\]

An alternative is to explicitly remove water from the atmospheric column:

\[
q_{new}=q-R\Delta t
\]

Then deposit it on the surface.

The most important spatial behaviors are:

- moisture decreases inland
- windward mountains receive more rain
- leeward areas become drier
- warm oceans generate more moisture
- cold air holds less moisture

Simplified orographic models demonstrate that even highly reduced formulations can reproduce the essential windward enhancement and leeward rain-shadow behavior.

---

# 22. Rainfall and Surface Water

Precipitation should enter a surface water budget.

\[
W_{surface,new}
=

W_{surface} +
R
-

E
-

Runoff
\]

Runoff can later be used to generate:

- rivers
- lakes
- wetlands
- groundwater-like moisture

This does not need to be solved at full hydrological fidelity during climate equilibrium.

The essential interaction is:

> precipitation increases available surface water, which increases evaporation and therefore feeds moisture back into the atmosphere.

---

# 23. Latent Heat

Phase changes transfer energy.

Condensation releases latent heat:

\[
Q_{latent}=L_vC
\]

Evaporation consumes latent heat:

\[
Q_{evap}=-L_vE
\]

Therefore:

\[
Q_{water}=L_v(C-E)
\]

and:

\[
T\leftarrow
T+
\frac{Q_{water}\Delta t}{C_{cell}}
\]

Exact values can be heavily parameterized.

The feedback itself is important:

> evaporation cools → condensation warms.

Latent heat is a real component of atmospheric energy balance and should not be completely removed if the goal is a coupled climate simulation.

---

# 24. Clouds

Clouds can initially be implicit.

Rather than maintaining detailed cloud microphysics, treat excessive atmospheric moisture as cloud/condensate potential.

Optional state:

\[
cloud = f(q,RH,uplift)
\]

Clouds can then modify:

- albedo
- solar absorption
- outgoing radiation
- precipitation probability

This should be considered a second-stage feature.

The first version should be able to produce convincing climate without explicit cloud simulation.

---

# 25. Ocean Heat

Ocean cells should act as thermal reservoirs.

Use:

\[
C_o\frac{dT_o}{dt}
=

Q_{solar}
-

Q_{outgoing}
-

Q_{evap} +
Q_{oceanTransport}
\]

The critical simplification is to make:

\[
C_o \gg C_{land}
\]

This automatically creates:

- cooler summers near oceans
- warmer winters near oceans
- weaker coastal temperature extremes
- stronger continentality inland

Explicit three-dimensional ocean circulation is unnecessary.

A simple horizontal diffusion/advection system is sufficient.

Live Stage 7 does not add `Q_oceanTransport` inside `relax_temperature`. Folding the tracer into the 768-iteration energy residual overwrites latent heat. The live term is a mixed-layer tracer along `derive_currents`, then a diagnostic imprint onto product T.

---

# 26. Ocean Currents

Ocean currents can initially use a simplified combination of:

- prevailing wind
- temperature gradient
- latitude
- coast geometry

For example:

\[
\mathbf{O}
=

K_W\mathbf{V}
-

K_T\nabla T_o
\]

Ocean currents then transport heat:

\[
Q_{ocean}
=

-\nabla\cdot
(C_o\mathbf{O}T_o)
\]

The exact ocean dynamics are not essential.

What matters is that large bodies of water redistribute heat.

Live Stage 7 realizes `Q_ocean` as flux-form advection of internal `T_ocean` (land faces no-flux). That divergence is not inserted as an energy residual; product T copies the mixed layer onto ocean cells and blends one land cell inland.

---

# 27. Seasonal Cycle

The simulation should not converge against a single fixed solar state.

Solar forcing should vary throughout an orbital year.

At minimum:

```text
season 0
season 1
season 2
season 3
```

A continuous seasonal phase is preferable.

For each timestep:

\[
S=S(\phi,\epsilon,\lambda,r)
\]

where:

- \(\phi\) = latitude
- \(\epsilon\) = axial tilt
- \(\lambda\) = orbital position
- \(r\) = orbital distance

Run enough cycles for the climate to stabilize into a repeating seasonal pattern.

The desired state is therefore not necessarily:

\[
State_{n+1}=State_n
\]

but rather:

\[
State_{n+1}\approx State_n
\]

after one complete orbital cycle.

---

# 28. The Climate Iteration

One climate timestep should perform the following operations.

```text
1. Calculate solar radiation.

2. Apply surface absorption.

3. Update surface and ocean temperature.

4. Calculate outgoing thermal radiation.

5. Calculate temperature gradients.

6. Update pressure.

7. Calculate pressure gradients.

8. Update wind using:
   - pressure gradient
   - Coriolis force
   - surface drag

9. Advect heat.

10. Calculate evaporation.

11. Advect atmospheric moisture.

12. Calculate atmospheric uplift.

13. Calculate saturation.

14. Condense excess moisture.

15. Generate precipitation.

16. Update surface water.

17. Apply latent heat.

18. Update ocean heat.

19. Apply optional diffusion / unresolved transport.

20. Record climate statistics.
```

Then repeat.

---

# 29. Convergence

The simulation should continue until the climate approaches equilibrium or seasonal equilibrium.

Track changes in major fields:

\[
\Delta_T=
\frac{1}{N}
\sum_i
|T_i^{n}-T_i^{n-1}|
\]

\[
\Delta_q=
\frac{1}{N}
\sum_i
|q_i^{n}-q_i^{n-1}|
\]

\[
\Delta_R=
\frac{1}{N}
\sum_i
|R_i^{n}-R_i^{n-1}|
\]

Stop when these values remain below configurable thresholds for a suitable number of iterations.

For seasonal simulation, compare equivalent points in successive years:

\[
\Delta_{year}
=

|State_{year,n}-State_{year,n-1}|
\]

This is preferable to blindly running an arbitrary number of iterations.

A maximum iteration count should still exist as a safety limit.

---

# 30. Numerical Stability

The equations are deliberately simplified, but numerical instability can still produce:

- exploding temperatures
- extreme winds
- negative humidity
- oscillating pressure
- infinite precipitation

Every update should therefore have physical bounds.

Examples:

```text
humidity >= 0
surface water >= 0
precipitation >= 0
temperature within configured planetary limits
wind speed <= configured maximum
pressure > 0
```

Transport should conserve quantities where appropriate.

For example:

> water moved from cell A to cell B must not disappear from the system.

Numerical stability is more important than mathematical elegance for this application.

---

# 31. What Must Be Physically Coupled

The following interactions are **essential**.

### Solar radiation → temperature

Without this, planetary parameters have little meaning.

### Temperature → pressure

Without this, atmospheric circulation becomes arbitrary.

### Pressure → wind

Without this, wind is procedural decoration.

### Rotation → wind

Without Coriolis effects, planetary rotation has little climatic consequence.

### Wind → heat transport

Without this, the equator-to-pole temperature gradient remains too static.

### Wind → moisture transport

Without this, rainfall cannot naturally follow atmospheric circulation.

### Water → evaporation

Without this, oceans cannot drive climate.

### Temperature → saturation

Without this, humidity has no meaningful precipitation threshold.

### Uplift → precipitation

Without this, mountains do not produce convincing rain shadows.

### Precipitation → water availability

Without this, the water cycle is disconnected.

### Evaporation / condensation → heat

Without latent heat, the water cycle does not participate in the energy balance.

These form the minimum convincing feedback network.

---

# 32. What Can Be Simplified Aggressively

The following systems do not need scientific fidelity.

## Atmospheric pressure magnitude

Relative pressure patterns matter much more than reproducing actual pascals.

## Wind speed

The model mainly needs believable:

- direction
- broad circulation
- relative strength
- seasonal movement

Exact meters-per-second values are unnecessary.

## Ocean circulation

A two-dimensional parameterized heat transport model is sufficient initially.

## Cloud physics

Use humidity and uplift instead of cloud microphysics.

## Vertical atmospheric structure

Represent it implicitly through:

- lapse rate
- uplift
- saturation
- pressure decrease with elevation

## Storm dynamics

Do not solve full cyclone physics.

## Radiation

Use simplified energy balance rather than spectral radiative transfer.

## Atmospheric chemistry

Compress greenhouse effects into a small number of parameters.

## Convection

Use parameterized heating/uplift rather than solving vertical fluid dynamics.

---

# 33. What Should Not Be Replaced With Noise

Procedural variation can still be useful, but it should not substitute for causal processes.

Avoid independently randomizing:

```text
temperature
rainfall
wind direction
humidity
biomes
storm locations
```

Noise may be used for:

- unresolved small-scale variation
- initial perturbations
- terrain roughness
- stochastic weather events

But generated noise should modify a physically generated field, not determine it.

For example:

```text
bad:

rain = noise()

better:

rain = simulated_precipitation + small_scale_variation()
```

---

# 34. Storms and Extreme Events

Storms should initially be treated as **derived events**, not as fundamental climate variables.

Calculate storm potential from conditions such as:

```text
warm surface
high humidity
pressure gradient
wind convergence
rotation
low vertical stability
```

A simplified storm potential could be:

\[
StormPotential =
F(T_{surface},q,\nabla P,Convergence,f)
\]

When the potential exceeds a threshold, generate an event.

For tropical cyclones, require sufficiently warm water and sufficient Coriolis strength.

For generic storms, pressure gradients and atmospheric moisture can be sufficient.

The exact thresholds should be tuned for visual and worldbuilding plausibility rather than Earth accuracy.

---

# 35. Climate Statistics

Do not derive biomes directly from individual simulation timesteps.

Accumulate statistics over many simulated seasons.

For every cell calculate:

```text
mean temperature
minimum temperature
maximum temperature
temperature variability
annual precipitation
seasonal precipitation
mean humidity
seasonal humidity
snow persistence
water availability
wind speed
prevailing wind
storm frequency
```

Additional useful metrics:

```text
dry season length
wet season length
temperature seasonality
precipitation seasonality
aridity index
continentality
growing-season length
```

The final climate classification should operate on these statistics.

---

# 36. Biomes

Biome generation should be downstream of climate.

Conceptually:

\[
Biome =
F(
Temperature,
Precipitation,
Seasonality,
Humidity,
Elevation,
WaterAvailability
)
\]

The biome system should therefore never need to know why a region is dry.

The climate simulation already established that:

```text
subtropical high pressure
+
descending dry air
+
distance from moisture
=
low precipitation
```

The biome classifier simply sees:

```text
high temperature
low precipitation
```

and produces an appropriate dry biome.

---

# 37. Parameter Hierarchy

The model should distinguish between **planetary parameters**, **physical coefficients**, and **artistic tuning parameters**.

## Planetary parameters

These fundamentally define the planet:

```text
star luminosity
orbital distance
orbital eccentricity
axial tilt
rotation period
atmospheric density
atmospheric heat retention
global ocean coverage
```

## Physical coefficients

These control model behavior:

```text
heat diffusivity
wind pressure coefficient
Coriolis coefficient
surface drag
evaporation rate
moisture transport rate
condensation rate
orographic uplift coefficient
latent heat coefficient
ocean thermal capacity
ocean heat coupling
ocean mixed-layer diffusivity
ocean heat advection
storm thermal-front scale
storm divergence kill
storm convergence scale
```

## Artistic tuning

These exist to compensate for model simplification:

```text
temperature amplification
precipitation amplification
rain-shadow strength
storm frequency
wind strength
ocean moderation
coastal ocean-heat blend
seasonal intensity
```

These categories should remain separate.

A planet parameter should not secretly compensate for a numerical parameter.

---

# 38. Recommended Implementation Strategy

Do not attempt the complete model simultaneously, and do not replace the existing climate product with a greenfield simulator.

The live implementation is `crates/daena-physical/src/climate.rs`, forced by `crates/daena-physical/src/planetary.rs`. Atlas and maps consume `ClimateField`. Later stages must upgrade that pipeline in place: keep the public fields, bump `CLIMATE_DERIVATION_VERSION` when semantics change, and couple processes that today run once and never feed back.

The existing entry point is `derive_current_climate`:

```text
build_geometry
        ↓
temperature_field          (energy-balance T, ice albedo, diffusion, flux-form heat advection, T/V coupling; two solstices)
        ↓
derive_winds               (internal P_base + thermal/elevation anomaly; closed-form geostrophic–drag V)
        ↓
derive_currents            (wind, rotation, SST gradient, gyres; internal T_ocean advected along them)
        ↓
derive_moisture_fields     (saturation-limited C, orographic V·∇h cooling of q_sat, leftover convergence; land E from W)
        ↓
runoff_fields              (diagnostic land runoff; hydrology input)
        ↓
classify_biomes
        ↓
derive_storms              (T, q, convergence, thermal |∇P|, Coriolis, solstice shear)
        ↓
extreme metrics            (drought / heat-wave / extreme rain from seasonal T/P)
```

Stages 1–5 replaced closed-form T, prescribed zonal winds, fraction-of-incoming rain, the leftover orographic rain fraction, and `LAND_MOISTURE_RECYCLE`. Pressure stays internal scratch. Stage 7 advects an internal ocean-temperature tracer along `derive_currents` and imprints it onto product T.

## Current baseline — already shipped

Treat the following as the starting surface, not as work to reimplement.

| Target process          | Current code                                                                                                                                         | Role today                                                                                     |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Planetary parameters    | `PlanetaryConfiguration`: luminosity, SMA, eccentricity, tilt, rotation, retained heat, global bond albedo                                           | Durable world config. `ClimateSettings.global_temperature_centi_c` remains an artistic offset. |
| Solar / radiative scale | `solar_base_centi_c`: insolation × `(1 − albedo)^(1/4)` scaled to an Earth equator base                                                              | Global scale only. Not per-cell `cos θ` insolation.                                            |
| Temperature             | `temperature_field`: latitude cooling, lapse, maritime blend, two solstice snapshots                                                                 | Diagnostic T. No `C ∂T/∂t`, no outgoing radiation, no heat transport.                          |
| Elevation cooling       | `altitude_lapse_centi_c_per_km`                                                                                                                      | Keep.                                                                                          |
| Ocean moderation        | Distance-to-ocean `maritime_factor`                                                                                                                  | Proxy for heat capacity. Not `C_o ≫ C_land`.                                                   |
| Circulation             | `derive_winds` / `circulation_flow`: Hadley / Ferrel / Polar from thermal equator; `omega_ratio`; seeded meanders; land roughness; mountain blocking | Coherent prevailing winds without pressure.                                                    |
| Moisture transport      | `transport_moisture`: iterative upwind until `Δ ≤ 1 mm`                                                                                              | The only iterated field.                                                                       |
| Evaporation             | `ocean_evaporation_mm` (SST + current speed); land E from RH/wind/`T` limited by internal `W`                                                        | Dry land does not emit ocean-like moisture.                                                    |
| Saturation / humidity   | Magnus `saturation_moisture_mm`; humidity = `q / q_sat` (0–1e6)                                                                                      | Product RH. Rain is condensed excess vapor plus leftover convergence.                          |
| Orography / rain shadow | `q_sat = saturation(T − K_U U_oro)`, `U_oro = max(0, V · ∇h)`; `orographic_precipitation_ppm` is artistic `K_U`                                      | `T_effective` is saturation-only. Lapse still cools product T.                                 |
| Runoff                  | `runoff_fields` from precipitation and hydrology preset                                                                                              | Diagnostic hydrology input. Climate `W` is internal `max(0, W + R − E − runoff)`.              |
| Seasons                 | Two-solstice year loop; product NH summer / winter T, wind, precipitation                                                                            | Annual T remains spin-up + latent; solstice T is that annual plus loop anomaly.                |
| Ocean currents          | `derive_currents`: wind coupling, Coriolis turn, geostrophy on air T, Sverdrup, western boundary                                                      | 2D surface gyres. Internal mixed-layer T is advected along currents and imprinted onto product T. |
| Biomes                  | `classify_biome_cell` from T, precipitation, humidity, aridity, elevation                                                                            | Already downstream of climate.                                                                 |
| Storms                  | `derive_storms`: SST, humidity, convergence, thermal `|∇P|`, Coriolis, solstice shear, fetch, tracks; drought / heat-wave / extreme-rain metrics from seasonal T/P | Climatology, not weather. Materialized storms stay `events.rs`. |

Seeded wind meanders and band moisture multipliers may remain as unresolved perturbation. They must not become the source of temperature, rainfall, or biomes.

## Constraints for every later stage

- Extend `derive_current_climate` and `ClimateField`. Do not add a second climate authority.
- Climate remains a disposable interpretation of accepted terrain. Derivation must not write elevation.
- Historical epoch offsets continue to apply after derivation (`with_global_temperature_offset` and history forcing).
- Keep inspectable product fields Atlas already samples: temperature, seasonal T, wind, currents, moisture, precipitation, humidity, aridity, biome, storms, runoff.
- Split new knobs using §37: planetary vs physical coefficients vs artistic tuning. Do not hide numerical coefficients inside planetary sliders.
- Prefer initializing the coupled state from the current analytic fields over discarding them.
- Do not grow `ClimateField` or `encode_climate` until a stage needs a new inspectable product. Internal scratch fields stay inside derivation.
- Qualitative tests in `climate.rs` are contracts (equator warmer than poles, rain shadows, land-only runoff). Exact centi-C fixtures are implementation details and may be retuned when T changes.
- A later stage’s physics is the contract. Do not distort new coefficients or diagnostics to keep a previous stage’s metric green. Update the old test. Stage 3 humidity is `q / q_sat` (0–1e6). Geostrophic `1/f` winds may strengthen (not weaken) wind-driven currents as rotation slows. Solstice-vector storm-shear milli thresholds may retune when wind scale changes.
- Each stage ships behind one `CLIMATE_DERIVATION_VERSION` bump. Cached climate is disposable; old cache files must fail validation rather than decode mixed semantics.

## Sequencing rule

Implement **one stage at a time**, in order. A stage is not started until the previous stage’s acceptance list is green and `CLIMATE_DERIVATION_VERSION` has been bumped for that stage.

A stage may land as more than one PR, but it has no “later leftover” physics. When the stage is done, every item in its **In this stage** list is implemented, tested, and wired through `derive_current_climate` and `with_winds_and_moisture_for_field`. Work listed under **Not this stage** belongs to a later numbered stage; do not pull it forward and do not leave in-stage work unfinished in order to start the next one.

Stage 1 shipped. Stage 2 shipped. Stage 3 shipped. Stage 4 shipped. Stage 5 shipped. Stage 6 shipped. Stage 7 shipped. Stage 8 shipped.

## Stage 1 — Coupled thermal model

Start from `temperature_field` and `solar_base_centi_c`. Keep the pipeline after T: winds, moisture, biomes, and storms still consume the temperature field.

**In this stage**

```text
initialize T from temperature_field
per-cell Q_solar from insolation, latitude (cos θ), global bond albedo
land / ocean / ice albedo from land mask and T < 0 (not biomes)
Q_outgoing = A + B T
Q_transport = heat diffusion to neighbors
C_cell ∂T/∂t = Q_solar − Q_outgoing + Q_transport
lapse as diagnostic altitude offset
iterate until mean |ΔT| is under a bound, with a max-iteration safety cap
```

Keep two solstice snapshots as analytic tilt/eccentricity offsets on the equilibrated annual field. Keep `maritime_factor` blend and continentality for those snapshots. Keep `ClimateSettings.global_temperature_centi_c` as artistic offset `G`. Keep `ClimateField` shape unchanged.

Add physical coefficients (not planetary sliders): heat diffusivity, outgoing `A`/`B`, land/ocean `C`, ice albedo, iteration cap / `ΔT` tolerance.

`C_land` vs `C_ocean` is required for a stable timestep. At annual equilibrium `∂T/∂t → 0`, so heat capacity does not produce coastal mildness. Do not remove `maritime_factor`. That is Stage 6.

**Not this stage:** heat advection by wind, pressure, saturation rain, surface-water budget, orbital-year loop, ocean-temperature transport, biome-dependent albedo, new `ClimateField` columns.

**Done when:** equator warmer than poles; `seafloor_depth_does_not_warm_ocean_air`; closer orbit / higher luminosity warms the annual field; altitude lapse cools land above sea level; ice albedo cools frozen cells relative to open water at the same latitude; energy-balance iteration converges or errors `NumericNonConvergent`; terrain unchanged; `encode_climate` / `decode_climate` round-trip aside from `CLIMATE_DERIVATION_VERSION`.

## Stage 2 — Pressure-driven wind and heat advection

Start from `derive_winds` / `wind_components`. Keep thermal-equator ITCZ, `hadley_edge_radians(omega)`, mountain blocking, land/ocean roughness, and the three snapshot winds. Pressure stays internal scratch unless inspect needs it.

**In this stage**

```text
P_base from existing Hadley / Ferrel / Polar bands (rotation sets cell width)
P = P_base − K_P (T − T_regional) − elevation scale height
G_P = −∇P
Coriolis f = 2Ω sin φ
drag K_D from ocean / land / mountains
V from the closed-form balance 0 = −∇P + f k̂×V − K_D V (diagnostic; no V iteration)
seeded meanders remain a capped perturbation
Q_transport = diffusion + upwind V·∇T
couple T and V for a bounded number of passes
```

`P_base` may encode planetary-scale easterlies/westerlies; `P_thermal` is the local anomaly. Do not ship this stage with prescribed zonal signs as the actual wind. Do not open a Navier–Stokes solver. The linear drag–Coriolis balance is algebraically the same as “relax V until residual 0”; there is no V residual metric and no `NumericNonConvergent` on wind.

Coupling Jacobi holds ice albedo `Fixed` from the T field at the start of that pass so live `T < 0` ice cannot snowball inside one energy-balance solve. Albedo therefore lags the spec’s live ice by one outer coupling pass; that lag is intentional.

Pressure and V are unsmoothed. Semi-implicit heat-advection diagonal plus `MAX_WIND_MILLI` are the only high-wavenumber controls.

**Not this stage:** moisture physics, saturation rain, new Hadley-width law, vertical structure, `ClimateField` pressure column.

**Done when:** `earth_like_winds_have_tropical_easterlies_and_midlatitude_westerlies`; `slow_rotation_expands_hadley_easterlies`; `northern_summer_shifts_itcz_north`; `mountains_block_zonal_wind`; Coriolis reverses across the equator and `f → 0` on the equator; wind clamped to `MAX_WIND_MILLI`; default `heat_advection_kj_m2_k` moves warmth downwind relative to Stage 1 diffusion-only T; a flipped thermal pressure anomaly reverses meridional V with `P_base` suppressed; `with_winds_and_moisture_for_field` restamps the same wind path; `climate_moisture_ranges_are_worldlike` on moisture/precipitation, not RH; `slower_rotation_changes_surface_currents` (not a weaken-currents fixture).

## Stage 3 — Saturation-limited moisture

Moisture must be carried by the pressure-driven wind.

Start from `transport_moisture`, `ocean_evaporation_mm`, `saturation_moisture_mm`. Keep iterative upwind, ocean sources, convergence rain, inland decay, and the orographic **fraction** (Stage 4 replaces that fraction). Keep `LAND_MOISTURE_RECYCLE = 0.40` (Stage 5 replaces that recycle).

**In this stage**

```text
E from water availability, T, RH, wind speed (ocean still uses SST/current)
C = K_C max(0, q − q_sat(T))
R from C plus existing convergence
q ← q − C
humidity product = q / q_sat (clamped 0–1e6)
latent heat: T ← T + L(C − E) / C_cell, then re-relax T/V within the same derivation
```

Retire `incoming * base_precipitation_ppm` as rain. Do not keep that fraction alongside excess-vapor removal. Remove the knob from `ClimateSettings`.

Humidity `q / q_sat` is a product change. Update biome thresholds, Atlas humidity style, Find Place humidity copy, `explain_biome`, and `PHYSICAL_WORLD_ROADMAP` in this stage so consumers match the new 0–1e6 meaning.

Old humidity was `q / (q + q_sat)`. Convert thresholds with `r / (1 − r)`: forest 350k → 550k, storm min 150k → 176k, storm full 450k → 818k. Maritime (arid shrubland) 700k converts to >1e6; retune to 850k so humid arid coasts remain reachable without requiring saturation.

`SATURATION_MOISTURE_PER_HPA = 90` so `q_sat` is comparable to `ocean_moisture_mm_per_year` at typical SST; 180 left `C = K_C max(0, q − q_sat)` unused. `CLIMATE_TRANSPORT_RELAXATION = 0.55` so doubled ocean-source Jacobi still converges inside the iteration cap. `latent_heat_coupling_ppm` scales `L(C − E)` because Budyko OLR already includes mean latent; full unscaled `L(C − E)` wrecks Stage 1 T. Outer moisture–T coupling under-relaxes latent (`MOISTURE_TEMPERATURE_RELAXATION`), holds V fixed during the T loop, then re-relaxes V once. It must converge on mean `|ΔT|` and mean `|Δq|` (`COUPLING_T_TOLERANCE_C`, `COUPLING_Q_TOLERANCE_MM`) or error `NumericNonConvergent` (one pass is diagnostic and skips that outer error). Inner moisture Jacobi still errors at 1 mm max delta at fixed T; the outer `|Δq|` bound is `|q(T_n) − q(T_{n−1})|` while T is still moving. After the last T/V update, run moisture once more so `q` / rain / humidity match final T. Epoch-0 cache still restamps climate; Stage 3 latent restamp is not byte-identical to present T, but mean T/q/rain stay close. If hydrology revises sea level, cached restamp starts again from present T+ΔT so it matches a single restamp at the final shoreline.

**Not this stage:** orographic `V·∇h` cooling, surface-water `W`, seasonal orbital loop.

**Done when:** `coastal_moisture_drives_interior_drying`; `warmer_sea_surface_increases_ocean_moisture`; `frozen_seas_evaporate_less_than_open_water`; `humidity_tracks_incoming_moisture_versus_saturation` on the new RH scale; rain shadows still hold via the leftover orographic fraction; condensation warms / evaporation cools T; `q >= 0`; precipitation `>= 0`; moisture and coupled T iteration converge or error `NumericNonConvergent`.

## Stage 4 — Orographic cooling

Cooling `q_sat` only changes rain if rain is excess vapor.

**In this stage**

```text
U_oro = max(0, V · ∇h)
q_sat = saturation(T − K_U U_oro)
retire the orographic precipitation fraction
reuse orographic_precipitation_ppm as artistic K_U
```

`T_effective` is only for saturation. Do not write it into `temperature_centi_c` (lapse already cools mountains). Never run fraction and `V·∇h` together.

`orographic_precipitation_ppm` keeps its Stage 3 name and 0…50e6 range as artistic `K_U`, not a leftover rain fraction. `U_oro = max(0, |V|/MAX_WIND_MILLI · ∇h)` using upwind height differences (m/m). `T_drop = min(25 °C, U_oro · ppm / 1000)`. Default 18e6 is 18 °C per 0.001 of normalized uplift: a 2 km ridge on 16×8 is a few °C; steep high-resolution slopes hit the 25 °C cap instead of crossing the Magnus pole at −243.5 °C. `T_effective` and `saturation_moisture_mm` clamp to [−80, 60] °C before Magnus `exp`. Product T may still move via Stage 3 latent `L(C − E)`, not via copying `T_effective`.

`cold_continental_interiors_can_be_cold_grassland` moved the 64×32 slab from rows 24–30 to 21–27. With `K_U = 0` the old band still classifies cold grassland; default `K_U` rains out on the 200 m escarpment, so inland `C` and latent heating fall and summer T at 24–30 drops below the tundra threshold. The contract is that a cold interior can be cold grassland, not that that polar band stays grassland.

**Not this stage:** surface-water budget, seasonal loop.

**Done when:** `ridge_creates_windward_precipitation_and_leeward_shadow`; leeward drier than windward at the same latitude; surface T lapse tests unchanged; `orographic_cooling_does_not_write_surface_temperature`.

## Stage 5 — Water feedback

Rain (including orographic) is the water source.

Do not create a second hydrology. Rivers, lakes, and drainage stay `hydrology.rs`. Climate `W` is an evaporative store that replaces `LAND_MOISTURE_RECYCLE`.

**In this stage**

```text
W ← max(0, W + R − E − runoff)
land E reads W
delete LAND_MOISTURE_RECYCLE
runoff_fields remains the hydrology input
```

Keep `W` internal unless inspect needs it. `runoff_mm_per_year` stays land-only.

`W` is an annual bucket, not a hydrology river. Land E is `E_potential · W / (W + E_potential)` (half-saturation is `E_potential`; not a planetary slider). The Jacobi updates `W` from the same relaxed `R` and `E` it stores. After `q` converges at 1 mm, precipitation is smoothed, then `W` is closed once from that smoothed `R` and `E` so the bucket’s runoff term uses the same rain `runoff_fields` publishes. Frozen land (`T ≤ 0`) and ocean keep `W = 0`. Inner residual stays `q`. Annual / solstice passes each start `W = 0`; carrying `W` is Stage 6. Wide interiors can drop to 0 rain; that is accepted here and is not retuned via a land-ocean recycle. Inland rivers then see that rain through hydrology’s climate runoff input.

**Not this stage:** orbital-year loop, ocean-temperature tracer, carried seasonal `W`.

**Done when:** `climate_is_deterministic_and_runoff_is_land_only`; wet cells keep evaporative supply after rain; dry interiors do not emit ocean-like moisture; `W >= 0`; frozen land stores no `W`; hydrology still consumes climate runoff.

## Stage 6 — Seasonal loop

This is the stage where `C_ocean ≫ C_land` replaces `maritime_factor` in **temperature**. Do not simulate daily weather. Use a small number of orbital samples (two solstices up to ~12) with carried T, q, V, and W. Converge year-to-year.

**In this stage**

```text
for year until Δ_year < bound or max years:
  for season sample:
    S(φ, ε, λ, r)
    energy-balance step (C now matters)
    update P, V from T
    moisture + W step
  compare to previous year
sample NH summer / winter product fields from the cycle
remove maritime_factor from temperature and from seasonal amplitude
put epoch offset G inside the energy equation
```

Two solstice samples (`δ = ±ε`). `λ` is those solstices. Perihelion longitude is unauthored, so `r` is not sampled at the solstices; eccentricity only scales the annual-mean `1/sqrt(1−e²)` already in `toa_mean_wm2`, including when tilt ≠ 0. Insolation weights are `S_annual · (1 + SEASONAL_INSOLATION_ANOMALY sin φ sin δ)` with `SEASONAL_INSOLATION_ANOMALY = 0.35` so the pair averages to the annual field. Each season takes one implicit Euler step (`dt = year/2`) with ice albedo frozen at the annual mask so `C_ocean ≫ C_land` lags the ocean (V is Stage 2 diagnostic; each season recomputes V from T, and the previous V only advects heat). Product annual T stays the spin-up + Stage 3 latent field because a two-point finite-step cycle mean is not annual energy equilibrium (no `maritime_factor` blend). NH summer/winter are that annual T plus the year-loop anomaly. Jacobi moisture carries `q` and `W` across seasons inside the loop. Product rain (annual and both solstices) is solved from zeros at the product T/V. Year-to-year Δ is the mean of summer and winter `|T(n)−T(n−1)|`. `seasonal_year_max` (default 12, allowed 1..=24) errors `NumericNonConvergent` if the cap is below two years or if that Δ stays above 2 °C. `maritime_factor_ppm` stays a distance diagnostic. `with_winds_and_moisture_for_field` restamps V/q from the caller T via `derive_winds` + `product_moisture` and does not re-relax T.

**Not this stage:** ocean-tracer heat transport along currents, storm retune.

**Done when:** `axial_tilt_creates_solstice_contrast`; `seasonal_precipitation_differs_when_tilt_is_large`; `solstice_winds_change_seasonal_precipitation`; `interiors_are_more_seasonal_than_oceans_without_maritime_blend`; `maritime_scale_does_not_change_temperature`; `year_loop_errors_when_year_cap_is_too_low`; epoch restamp keeps `with_global_temperature_offset`.

## Stage 7 — Ocean heat transport

Do not rebuild gyres. Use `derive_currents` as-is.

**In this stage**

```text
ocean-temperature tracer advected / diffused along existing currents
land faces of that tracer are no-flux
mixed layer imprinted onto product T after latent coupling
geostrophy still reads air T
```

Keep ocean T internal unless inspect needs it. Currents stay zero on land and in inland sinks.

An internal mixed-layer tracer `T_ocean` is initialized from the energy T on `ocean_mask` basins, carried from annual T/V into the year loop, and advected/diffused along existing gyres (land faces are no-flux), nudged toward air T by `K = ocean_heat_coupling_milli_wm2_per_c` (default 12 W m⁻² K⁻¹). Diffusivity and advection scales are `ocean_heat_diffusivity_ppm` and `ocean_heat_advection_ppm`. After latent coupling, product T imprints that mixed layer onto ocean cells and a one-cell coastal blend (`ocean_coast_blend_ppm`, artistic), so a western-boundary coast warms because heat was carried, not because `maritime_factor` was blended. The imprint is a diagnostic overwrite, not a conservative flux into the energy residual. `K = 0` skips the tracer and the imprint. Seasons carry `T_ocean` with one mixed-layer step; they do not re-relax T against SST. Year-to-year Δ includes basin `|ΔT_ocean|` with the same 2 °C bound as air. `derive_currents` stays as-is (geostrophy still reads air T). Putting the tracer inside the 768-iteration energy relax overwrites latent heat, so the flux enters the product by imprint rather than by re-solving Stage 1. `with_winds_and_moisture_for_field` does not re-run the tracer or recouple T.

**Not this stage:** storm retune, new current solver.

**Done when:** `western_boundary_current_is_stronger_than_the_basin_interior`; `ocean_currents_are_zero_on_land`; `enclosed_seas_get_wind_driven_currents`; `western_boundary_coast_is_warmer_than_the_opposite_coast` (east coast of a north–south continent warmer than the west coast at the same latitude; the contrast shrinks when `K = 0`; equal `maritime_factor_ppm` on both coasts).

## Stage 8 — Storms and extremes

Baseline `derive_storms` already existed; this stage finishes climatology against the coupled fields.

**In this stage**

```text
retune suitability from T, q, convergence, pressure gradient, Coriolis
keep shear as the solstice wind-vector difference (not vertical shear); Stage 2 raised `STORM_SHEAR_KILL_MILLI` 12 000 → 20 000 because geostrophic solstice ΔV exceeds the prescribed-wind kill — this stage must re-justify or replace that threshold
derive drought / heat-wave / extreme-rainfall potential from Stage 6 statistics
```

Do not run daily weather across history. Materialized storms stay authored samples in `events.rs`.

`classify_storm_cell` multiplies SST, humidity, latitude/Coriolis, `(1 − shear)`, convergence, thermal `|∇P|`, proximity, current, and fetch. SST is `max(annual, summer, winter)`. Convergence is the most-convergent of those three `wind_divergence_ppm` snapshots. Thermal `|∇P|` is the **max** ocean-neighbor `|ΔT|` across the same three snapshots (a seasonal front can kill genesis), scaled by cell spacing — a proxy for Stage 2 `P_thermal = −K_P ΔT`, not a stored P field. Land–sea jumps are ignored so coasts are not zeroed by the shoreline contrast. An isolated ocean cell with no ocean neighbor has `|∇P| = 0` (no front to measure; fetch already damps 1-cell ponds). Convergence factor is `(1 + 0.25 conv) × (1 − 0.70 div)` clamped to `[0, 1.25]`: strong low-level convergence may raise suitability 25% before the 1e6 cap; divergence damps. Physical knobs on `ClimateSettings`: `storm_pressure_gradient_start_centi` / `kill_centi` (default 2 °C / 9 °C), `storm_divergence_start_ppm` / `kill_ppm` (40k / 350k), `storm_convergence_full_ppm` (300k). Shear stays the solstice wind-vector difference. `STORM_SHEAR_START_MILLI` stays **2 500** (unchanged from Stage 2). `STORM_SHEAR_KILL_MILLI = 12_000` (`1.2 × MAX_WIND_MILLI`): that ΔV is reachable when solstice winds reverse at a large fraction of the clamp, so high-shear midlatitudes actually zero genesis. The Stage 2 value `20_000` was the vector-difference ceiling (`2 × MAX_WIND_MILLI`) and never fired.

Drought / heat-wave / extreme-rainfall potentials are functions of annual and solstice T, P, and aridity (`classify_extreme_cell`). Drought ramps aridity from grassland (`GRASSLAND_ARIDITY_PPM` 200k) to desert (`DESERT_ARIDITY_PPM` 800k), dryness from 80 mm to forest precip (`FOREST_PRECIPITATION_MM` 500), and driest-solstice rain 20–200 mm. Heat-wave ramps peak T 22–38 °C plus solstice amplitude 4–25 °C, with ocean weight 0.25 because `C_ocean ≫ C_land` lags. Extreme rain ramps peak P 600–2500 mm (above forest) times monsoon contrast 80–800 mm. They are published as `ClimateMetrics` means, not `ClimateField` columns and not prognostic state. `with_winds_and_moisture_for_field` restamps storms and those metrics. `storm_corridors_follow_continental_heat_and_ocean_fetch` samples row 9 (south flank of the 32×16 continent): after coupled genesis, row 6 (north edge) is outside the surviving band on the east side, so east-fetch vs west-lee is checked where genesis still exists. `CLIMATE_DERIVATION_VERSION` is 10.

**Not this stage:** anything from Stages 1–7.

**Done when:** tropical ocean genesis; tracks toward land; colder epoch reduces suitability; storm metrics remain bounded; extreme-event fields are statistics of climate, not new state variables.

## Cross-stage traps

| Trap                                | Rule                                                                                                           |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Skipping ahead                      | Stage N+1 starts only after Stage N acceptance is green                                                        |
| Heat capacity at annual equilibrium | C does not replace `maritime_factor` before Stage 6                                                            |
| Orographic double count             | Fraction lives in Stage 3; Stage 4 replaces it with `V·∇h`; never both                                         |
| Uplift vs lapse                     | `T_effective` does not write surface T                                                                         |
| Two water authorities               | Climate `W` ≠ hydrology lakes/rivers                                                                           |
| Schema                              | `empty_climate` / both `synthetic_climate` helpers / `encode_climate` stay in lockstep if `ClimateField` grows |
| Restamp                             | `with_winds_and_moisture_for_field` restamps V/q from the caller T via `derive_winds` + `product_moisture`. It must not re-run the year loop or recouple T. Epoch offsets apply after derivation. |
| Ocean tracer in energy relax        | Do not fold `T_ocean` into the 768-iteration Stage 1 relax; that overwrites latent heat. Advect the tracer, then imprint onto product T. |

---

# 39. Essential vs Optional Complexity

| System                      | Importance            | Recommended treatment       |
| --------------------------- | --------------------- | --------------------------- |
| Solar radiation             | Essential             | Analytical                  |
| Albedo                      | Essential             | Parameterized               |
| Surface temperature         | Essential             | Energy balance              |
| Heat diffusion              | Essential             | Simple numerical diffusion  |
| Elevation cooling           | Essential             | Lapse approximation         |
| Ocean thermal inertia       | Essential             | Large heat capacity         |
| Pressure gradient           | Essential             | Derived field               |
| Rotation/Coriolis           | Essential             | Simplified vector force     |
| Wind                        | Essential             | Parameterized momentum      |
| Heat advection              | Essential             | Simplified transport        |
| Evaporation                 | Essential             | Bulk approximation          |
| Humidity                    | Essential             | Conserved scalar            |
| Saturation                  | Essential             | Clausius–Clapeyron / lookup |
| Condensation                | Essential             | Excess-vapor removal        |
| Rainfall                    | Essential             | Condensation + uplift       |
| Orographic uplift           | Essential             | \(V\cdot\nabla h\)          |
| Water storage               | Important             | Simple surface budget       |
| Latent heat                 | Important             | Simple energy feedback      |
| Seasons                     | Important             | Time-varying solar forcing  |
| Ocean currents              | Useful                | 2D approximation            |
| Clouds                      | Useful                | Parameterized               |
| Storms                      | Optional              | Threshold/event model       |
| Detailed ocean dynamics     | Unnecessary initially | Ignore                      |
| Atmospheric chemistry       | Unnecessary initially | Abstract                    |
| 3D atmosphere               | Unnecessary initially | Ignore                      |
| CFD/Navier–Stokes           | Unnecessary           | Do not implement            |
| Detailed cloud microphysics | Unnecessary           | Ignore                      |
| Exact weather prediction    | Unnecessary           | Ignore                      |

---

# 40. Core Design Principle

The simulation does not need to be scientifically correct.

It needs to be **causally convincing**.

A player should be able to look at a generated world and infer relationships such as:

```text
Why is this desert here?
→ persistent dry circulation + continentality + rain shadow

Why is this region wet?
→ moisture transport + warm ocean + uplift

Why is this coast mild?
→ high thermal capacity of nearby ocean

Why are these mountains snowy?
→ altitude + low temperatures

Why is this area stormy?
→ strong gradients + moisture + favorable rotation

Why does the southern hemisphere behave differently?
→ reversed Coriolis direction

Why does this planet have extreme seasons?
→ high axial tilt
```

Those causal relationships are more valuable than accurately reproducing any particular terrestrial numerical value.

The desired final system is therefore:

\[
\boxed{
Planetary\ Parameters
\rightarrow
Energy\ Balance
\rightarrow
Temperature
\rightarrow
Pressure
\rightarrow
Atmospheric\ Circulation
\rightarrow
Heat/Moisture\ Transport
\rightarrow
Precipitation
\rightarrow
Surface\ Feedback
\rightarrow
Climate
}
\]

repeated over simulated time until the planet reaches a stable or seasonally repeating climate.

The simulation should generate **climate as an emergent property of interacting fields**, with procedural noise restricted to phenomena that the simplified physics cannot or should not explicitly represent.
