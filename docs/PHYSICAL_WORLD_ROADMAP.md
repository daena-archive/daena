# Daena Physical World — Future Product Roadmap

## Status

**Type:** Product specification / roadmap
**Scope:** Future capabilities extending Daena's native physical world, derived environmental data, and authored map layers
**Non-goal:** This document does not prescribe implementation architecture, algorithms, storage formats, or delivery dates.

This roadmap builds on the current map, physical-world, and Atlas product
boundaries defined by [`MAPS.md`](./MAPS.md), together with the platform and AI
boundaries in [`PLUGIN_PLATFORM_PLAN.md`](./PLUGIN_PLATFORM_PLAN.md) and
[`AI_INTEGRATION.md`](./AI_INTEGRATION.md).

The current physical world already provides immutable elevation/bathymetry, tectonic causes, hydrology, a simplified climate model, historical climate/sea-level derivation, earthquake/volcanic hazards, and optional materialized natural events. Future features should extend those capabilities without turning the accepted physical map into a continuously mutable terrain simulation.

---

# 1. Product Principles

## 1.1 The physical world remains authoritative and stable

The accepted physical map represents the underlying planet. Its base terrain must not be rewritten merely because the user changes the viewed year or because an earthquake, eruption, storm, or other event occurs.

Historical variation should normally come from derived state such as:

- climate;
- water and ice;
- coastlines;
- rivers and lakes;
- biomes;
- natural-event history;
- authored infrastructure and political/cultural layers.

Large earthquakes do **not** reshape the canonical landmass in this roadmap.

## 1.2 Physical causes should precede visible classifications

Where practical, Daena should derive world characteristics from upstream causes rather than paint plausible-looking results independently.

For example:

`planetary parameters -> solar heating -> temperature -> winds -> moisture -> precipitation -> biomes`

and:

`tectonics -> hazard -> materialized natural event -> Timeline/Lore consequences`

This does not mean Daena must become a scientific simulator. The goal is internally coherent worldbuilding.

## 1.3 Derived data is disposable; authored decisions are durable

Temperature, precipitation, wind, current, biome classification, hazard fields, search indices, and similar products may be recomputed.

A road accepted by the user, a layer they created, a materialized historical event, or a Lore/Timeline connection is authored world data and should survive derivation changes.

## 1.4 Epoch matters

Any feature that depends on climate, water, coastlines, ice, biomes, hazards, or infrastructure should be able to state which point in physical or authored history it represents.

A location that is suitable for settlement at one epoch may be underwater, glaciated, arid, or inaccessible at another.

## 1.5 Atlas consumes the world; it does not redefine it

Atlas may visualize and refine the physical world's presentation at much higher detail. It may render climate, biomes, roads, currents, and other layers, but Atlas should not become a second authority for physical truth.

---

# 2. Roadmap Summary

| ID  | Feature                                     | Group        | Standalone?        | Primary dependencies                            |
| --- | ------------------------------------------- | ------------ | ------------------ | ----------------------------------------------- |
| G1  | Planetary and orbital configuration         | Generation   | Partially          | Existing physical-world generation              |
| D1  | Solar-driven temperature and seasonality    | Derived data | No                 | G1                                              |
| D2  | Global wind circulation                     | Derived data | No                 | G1, D1                                          |
| D3  | Ocean currents                              | Derived data | No                 | D1, D2, hydrology/coastline                     |
| D4  | Moisture, humidity, and rainfall upgrade    | Derived data | Incrementally      | D2; D3 improves it                              |
| D5  | Biome classification                        | Derived data | No                 | D1, D4                                          |
| D6  | Storm and hurricane suitability/events      | Derived data | No                 | D1-D4                                           |
| D7  | Extended natural-event consequences         | Derived data | Partially          | Existing hazards; some effects depend on D1-D4  |
| D8  | Find Place / world search                   | Derived data | Yes, incrementally | Existing physical fields                        |
| L1  | Physical and climate visualization layers   | Layer        | Incrementally      | Corresponding derived field                     |
| L2  | Suggested road routing and persistent roads | Layer        | Yes                | Existing terrain/hydrology; D5 improves routing |
| L3  | Epoch-aware road history                    | Layer        | No                 | L2, Timeline integration                        |
| L4  | Landmass selection for authored layers      | Layer        | Yes                | Existing elevation/sea-level connectivity       |
| L5  | Natural-event map presentation              | Layer        | Yes                | Existing/materialized natural events            |

Dependencies describe product dependencies, not required implementation boundaries.

---

# 3. Generation-Related Milestones

## G1. Planetary and Orbital Configuration

### Goal

Give Daena's physical world a coherent planetary context so climate does not depend only on abstract global-temperature controls.

The physical globe/world configuration should allow the author to describe the planet's relationship with its star and its own rotation.

### Desired product behavior

The author should be able to configure a small set of meaningful planetary parameters such as:

- star energy output or an equivalent simplified stellar setting;
- mean distance from the star;
- orbital eccentricity;
- axial tilt;
- rotation period;
- orbital period or equivalent year length;
- planet size and gravity where they materially affect later systems;
- atmospheric characteristics relevant to retained heat;
- surface/ocean reflectivity where exposed at an appropriate level.

The UI should distinguish between:

- **simple controls**, appropriate for most worldbuilders;
- **advanced controls**, for authors intentionally designing unusual planets.

Daena should avoid presenting mutually dependent physical values as unrelated arbitrary sliders when doing so would create nonsensical configurations. The product may derive secondary values from a smaller set of primary choices.

### Why it is desired

The current climate can create useful hot/cold and wet/dry variation, but it does not yet have a real stellar/orbital cause. Planetary inputs provide a stable foundation for:

- believable latitudinal heating;
- seasons;
- polar/equatorial differences;
- circulation;
- ocean-current behavior;
- storm zones;
- more reliable biome classification.

### Standalone status

**Partially standalone.**

Planetary settings can be stored and displayed before the full climate upgrade exists, but their principal product value appears when downstream climate derivations consume them.

### Considerations

- The UI should not require astronomy knowledge for ordinary use.
- Presets such as Earth-like, low-tilt, high-tilt, slow-rotating, or close-orbit may be useful later.
- Extreme configurations should be allowed when they remain within supported model bounds.
- Daena should describe results as generated world physics, not as precise scientific prediction.
- Existing worlds need a stable fallback/default planetary profile so adding this feature does not invalidate them.

### Daena ecosystem interaction

**Atlas:** Uses resulting climate products for high-resolution biome, temperature, precipitation, ice, and terrain presentation.

**Timeline:** Planetary settings are long-lived world properties. Seasonal/calendar integration may later use orbital period, but this should not silently redefine authored calendars.

**Lore:** Authors may link unusual planetary characteristics to setting lore.

**Languages:** No direct dependency. Language modules may eventually name astronomical bodies or seasons.

**AI:** AI may explain a configuration, suggest presets from user intent, or summarize likely consequences. It should not become the authoritative physics calculator.

**Plugins:** Future plugins may consume shared planetary properties for astronomy, calendars, ecology, or speculative-world systems.

**Git:** Accepted planetary settings are meaningful world configuration and should be revisionable. Disposable climate caches should not become noisy project history.

---

# 4. Derived-Data Milestones

## D1. Solar-Driven Temperature and Seasonality

### Goal

Replace the current abstract global-temperature forcing as the only upstream heat source with temperature fields influenced by the configured planet and star.

### Desired product behavior

Daena should derive temperature using the planetary configuration while retaining the useful effects already supported by the current climate model:

- latitude;
- altitude;
- maritime moderation;
- historical climate variation.

The product should support meaningful seasonal contrast where axial tilt and orbital configuration justify it.

A future climate view should be able to answer questions such as:

- Which areas are warmest or coldest?
- How much does a region vary across the year?
- Where is permanent freezing plausible?
- Which high-altitude regions remain cold despite low latitude?

### Standalone status

**Depends on G1.**

It can be delivered before winds, currents, advanced rainfall, or biome upgrades.

### Considerations

- A useful climate approximation is preferred over a computationally expensive atmospheric simulation.
- Historical climate forcing must remain compatible with the existing epoch model.
- Seasonal detail should be bounded. A few representative seasonal states may be preferable to continuous day-by-day weather.
- Existing global-temperature controls may remain as a worldbuilding adjustment or greenhouse-like parameter rather than disappear entirely.

### Ecosystem interaction

Temperature becomes a shared input for search, biomes, storms, ice, Lore context, Atlas styles, and compatible plugins.

---

## D2. Global Wind Circulation

### Goal

Upgrade the current broad prevailing-wind behavior into an explicit, inspectable global wind field influenced by planetary rotation and temperature differences.

### Desired product behavior

Daena should derive prevailing winds for the selected epoch and, where supported, season.

The author should be able to inspect:

- wind direction;
- relative or estimated strength;
- major circulation bands;
- regions where wind converges or diverges;
- the effect of mountains and coastlines where represented by the model.

Wind should become an input to moisture transport rather than only an invisible internal mechanism.

### Why it is desired

An explicit wind field enables:

- more coherent rainfall;
- rain shadows;
- monsoon-like seasonal behavior in future iterations;
- desert placement;
- sailing/trade analysis in future systems;
- wildfire/storm extensions;
- ocean-current forcing;
- hurricane plausibility.

### Standalone status

**Depends on G1 and D1.**

It does not depend on ocean currents.

### Considerations

- This is a climate approximation, not computational fluid dynamics.
- Interactive display should emphasize prevailing conditions rather than imply exact local weather.
- Rotation speed should visibly affect circulation at the scale Daena claims to support.
- The system must remain useful on the existing coarse physical grid.

### Ecosystem interaction

**Atlas:** Can render wind arrows, streamlines, or climate maps.

**Timeline:** Different epochs may produce different prevailing patterns.

**Lore:** Can support descriptions of prevailing winds, dangerous sea routes, seasonal winds, and climate-regional identity.

**Languages:** Named winds may be authored as normal world knowledge; Daena should not require the Language module.

**Plugins:** Agriculture, sailing, wildfire, ecology, or trade plugins may consume wind information if exposed.

**AI:** Can turn structured wind/climate data into explanations, but should not invent wind values.

---

## D3. Ocean Currents

### Goal

Generate coherent surface-ocean circulation from the planet's climate, winds, rotation, and continental arrangement.

### Desired product behavior

For each supported epoch, Daena should be able to derive major current direction and broad relative strength.

Currents should react to:

- ocean and continent geometry;
- prevailing winds;
- planetary rotation;
- temperature differences where supported;
- large-scale basin connectivity.

The first product iteration should focus on major current systems rather than detailed deep-ocean circulation.

### Why it is desired

Ocean currents can materially improve:

- coastal temperature moderation;
- wet/dry coastal contrast;
- biome plausibility;
- storm/hurricane suitability;
- sea-route context;
- future marine ecology and migration systems.

### Standalone status

**Depends on D1 and D2**, plus existing land/ocean geometry.

It is not required before Find Place or roads can exist.

### Considerations

- Surface circulation is sufficient for the initial feature.
- Current vectors should not imply precision beyond physical-map resolution.
- Current behavior may change as coastlines change between epochs.
- Deep thermohaline circulation is a future extension, not an initial requirement.
- Surface currents are annual per epoch. Seasonal monsoon current fields are out of scope because D3 is epoch-level, not a second seasonality milestone.
- Moisture, biomes, storms, Find Place, and sea-route context consume currents in D4–D8 / Milestone C; D3's product is the current field itself.

### Ecosystem interaction

Atlas can render current lines or arrows. Search can eventually filter for current-adjacent climate conditions. Plugins may use currents for shipping, marine ecology, drift, or migration.

---

## D4. Moisture, Humidity, and Rainfall Upgrade

### Goal

Extend the existing moisture-transport system so rainfall and humidity respond to the improved temperature and circulation model.

### Existing baseline

Daena already supports broad prevailing-wind moisture transport, coastal-to-interior drying, orographic precipitation, and rain shadows. This milestone should evolve that system rather than replace it with an unrelated climate generator.

### Desired product behavior

Daena should provide inspectable derived fields for:

- precipitation;
- atmospheric or environmental humidity;
- dryness/aridity;
- runoff-relevant moisture;
- seasonal variation where supported.

The model should continue to produce:

- wet windward mountain slopes;
- dry rain shadows;
- dry continental interiors;
- wetter maritime regions.

Improved winds and currents should make these patterns more geographically coherent.

### Standalone status

**Incremental.**

The existing climate system already provides a foundation. D2 should precede the major upgrade; D3 may improve results later without blocking the first iteration.

### Considerations

- Humidity is remaining atmospheric moisture versus local saturation at annual temperature, shown as a percent of saturation — not a weather-station relative-humidity reading.
- Aridity is evaporative demand unmet by rainfall (potential evapotranspiration from temperature versus precipitation). Inspect uses humid / sub-humid / semi-arid / arid labels.
- Annual precipitation still feeds hydrology and runoff. Seasonal rainfall is inspectable from solstice wind fields.
- Humidity and aridity stay annual. Seasonal moisture passes reuse annual surface currents (D3 is epoch-annual, not monsoon currents).
- Orographic rain uses both zonal and meridional upslope, weighted by local wind.
- Ocean evaporation scales with sea-surface temperature and surface-current speed. Frozen seas evaporate little.
- Climate derivations should not modify accepted terrain.
- Atlas rainfall, humidity, and aridity styles tint land from those fields. Extra Atlas climate layers remain L1.

### Ecosystem interaction

This field becomes central to biomes, storms, world search, agriculture plugins, Atlas climate rendering, and AI-generated setting explanations.

---

## D5. Biome Classification

### Goal

Produce biome regions from physical climate conditions instead of treating biomes as manually painted or noise-driven world decoration.

### Desired product behavior

Daena should classify land into understandable biome categories using available physical conditions such as:

- temperature;
- precipitation;
- humidity/aridity;
- elevation;
- seasonality;
- ice/permanent freezing;
- potentially ocean-current influence.

Initial categories should remain broad and explainable, for example:

- permanent ice;
- tundra;
- cold grassland/steppe;
- temperate grassland;
- arid/desert;
- shrubland;
- temperate forest;
- tropical or very wet forest;
- alpine/highland variants where justified.

The user should be able to inspect **why** a location received a classification.

### Why it is desired

Biome classification turns climate fields into directly useful worldbuilding information and provides a common input for:

- settlement placement;
- travel;
- roads;
- cultures;
- ecology;
- agriculture;
- Atlas presentation;
- world search.

### Standalone status

**Depends primarily on D1 and D4.**

D3 improves coastal realism but need not block an initial biome system.

### Considerations

- Biomes remain derived interpretations, not immutable physical truth.
- Classification should be versioned so improving thresholds does not pretend old results were canonical.
- Authors should be allowed to override presentation with authored layers without rewriting the physical model.
- Humidity and aridity both feed classification. Permanent-ice biome is climate freeze, distinct from hydrology ice cover.
- Inspect should state the winning rule plus the temperature, elevation, rain, humidity, and aridity that applied.
- Ocean-current influence is already in moisture via SST/current evaporation; classification does not add a second current rule.
- Find Place biome filters, road traversal costs, and extra Atlas climate layers remain later Milestone C / L1 work.
- Future ecology plugins may provide alternative biome schemes.

### Ecosystem interaction

**Atlas:** Primary source for biome rendering.

**Lore:** Regions, cultures, creatures, resources, and settlements can reference biome context.

**Timeline:** A location's biome may change across very long climate history. The system should avoid silently rewriting authored cultural history when this happens.

**Languages:** Biome names used by cultures can exist independently from the physical classification.

**AI:** Useful for explaining ecological implications and suggesting worldbuilding consequences from structured biome data.

**Plugins:** Plugins may provide alternative classifications or derive ecology from the core climate fields.

---

## D6. Storm and Hurricane Suitability

### Goal

Allow climate to define where severe storms are plausible, then optionally materialize individual storms as historical events.

### Desired product behavior

Daena should distinguish between:

1. **storm suitability / climatology** — derived environmental data; and
2. **a specific storm** — durable world history once the user accepts/materializes it.

For tropical-cyclone-like systems, Daena should consider available factors such as:

- warm ocean conditions;
- moisture;
- latitude/rotation constraints;
- prevailing winds;
- wind shear where the model eventually supports an approximation;
- proximity and track toward land.

The product may initially expose:

- potential formation zones;
- broad storm frequency;
- plausible track corridors;
- relative intensity potential.

A user may then request one or more generated storm events over a chosen historical interval.

### Standalone status

**Depends on D1-D4.**

It is intentionally later than basic climate.

### Considerations

- Daena should not continuously simulate daily weather across 200,000 years.
- Generated storms should be sparse, bounded worldbuilding events.
- A materialized event must remain stable even if the future storm model changes.
- Storm effects on settlements, roads, or Lore should not be applied silently without author acceptance.

### Ecosystem interaction

**Timeline:** Natural home for accepted storms.

**Lore:** May link disasters, migrations, famines, legends, or political consequences.

**Maps:** Storm tracks and affected regions can be presented temporarily or as historical overlays.

**AI:** May draft a narrative summary from accepted event facts; the structured event remains authoritative.

**Plugins:** Plugins may add storm consequences for agriculture, trade, ecology, or demographics.

---

## D7. Extended Natural-Event Consequences

### Goal

Build on Daena's existing earthquake and volcanic hazards/materialized events so natural events can cause physically plausible secondary effects without mutating canonical terrain.

### 7.1 Tsunamis

A submarine earthquake should **not automatically create a tsunami**.

When an accepted earthquake has conditions capable of strongly displacing the water column, Daena may derive a tsunami event or tsunami potential.

Potential outputs:

- origin area;
- affected coastlines;
- relative severity;
- approximate arrival ordering or travel time where later supported;
- map overlay of exposed coasts.

Other future tsunami sources may include volcanic collapse or major submarine landslides.

### 7.2 Volcanic climate forcing

A sufficiently large eruption may create a temporary atmospheric forcing event.

Possible derived consequences:

- reduced incoming sunlight;
- temporary global or regional cooling;
- altered precipitation;
- temporary biome/climate stress.

The event should have a bounded duration and feed into climate history rather than permanently changing the planet.

### 7.3 Earthquake consequences

Earthquakes may produce event-level consequences such as:

- destructive shaking;
- landslides;
- infrastructure damage;
- tsunami potential.

They do not reshape the canonical physical map in this roadmap.

### Standalone status

**Partially standalone.**

Tsunami potential can build on existing bathymetry and earthquake events without the wind/current roadmap. Volcanic climate effects require the climate system to accept temporary forcing.

### Considerations

- Consequences should be proposals or derived facts, not silent destructive mutations of authored entities.
- The author should retain control over whether a settlement, road, state, culture, or character is considered affected.
- Event severity should be explainable from the generated physical causes.
- Durable events keep their identity and provenance after later model upgrades.

### Ecosystem interaction

**Timeline:** Primary historical representation.

**Lore:** Links disasters to places, peoples, institutions, religions, migrations, and stories.

**Roads:** A Timeline event may mark a road damaged, closed, abandoned, or rebuilt.

**AI:** Can propose plausible narrative consequences from accepted structured effects, but should not silently change world canon.

**Plugins:** Domain plugins could add agriculture, population, economy, disease, or ecology consequences.

**Git:** Materializing or editing an event is authored history and should appear as a meaningful project revision; recalculating hazard fields should not.

---

## D8. Find Place / World Search

### Goal

Let an author ask the world itself:

> "Where could a place like this exist?"

rather than manually scanning the map.

### Desired product behavior

The user opens **Find Place** and specifies one or more criteria.

Initial criteria should include any already available physical fields, such as:

- minimum/maximum altitude;
- temperature range;
- precipitation;
- humidity or dryness when available;
- biome;
- latitude;
- slope or terrain ruggedness;
- distance to coast;
- distance to river/lake/fresh water;
- landmass;
- earthquake hazard;
- volcanic hazard;
- current epoch.

Future criteria may include:

- wind;
- ocean currents;
- storm exposure;
- growing conditions;
- road access;
- travel time;
- distance to authored settlements or political regions;
- plugin-provided shared environmental properties.

### Results

Daena should not primarily return thousands of matching grid cells.

It should identify useful **candidate areas** and rank or group them.

Each candidate should explain why it matches, for example:

- average/range of relevant properties;
- criteria satisfied;
- criteria near the requested boundary;
- notable risks;
- nearest relevant water/coast/road where requested.

The user should be able to:

- focus the map on a candidate;
- compare candidates;
- pin a location;
- create or link an entity at the chosen location;
- optionally turn the result into an authored region/layer later.

### Epoch awareness

Find Place must search the world at a specific epoch whenever queried fields are time-sensitive.

The same query at another epoch may produce different results due to:

- sea level;
- ice;
- temperature;
- precipitation;
- rivers/lakes;
- biomes;
- historical roads.

### Why it is desired

This converts the physical generator from a visualization system into a worldbuilding assistant.

Examples:

- Find a warm, wet lowland suitable for a dense farming civilization.
- Find a cold mountain valley close to fresh water.
- Find a dry plateau at least 500 km from the coast.
- Find a low-hazard coastal location for a capital.
- Find a tropical coast exposed to hurricanes.
- Find a mountain pass connecting two existing settlements.

### Standalone status

**Yes, incrementally.**

An MVP can use the physical fields Daena already has today. New criteria appear automatically as later climate and infrastructure features become available.

It does **not** need to wait for the full climate roadmap.

### Considerations

- Exact thresholds and soft preferences should be distinguished.
- Candidate ranking should explain trade-offs rather than hide them behind one opaque score.
- Missing derived fields should simply make their criteria unavailable.
- Search should not imply every matching place is culturally or politically suitable.
- Results are suggestions, not mutations.

### Ecosystem interaction

**Lore:** A chosen result can be linked to or create a place/entity through normal Daena workflows.

**Timeline:** Search can target a historical epoch.

**Roads:** Future search can include accessibility and existing infrastructure.

**Languages:** No dependency. Once a place is authored, normal naming/language features apply.

**AI:** AI can translate a natural-language request into structured search filters or explain candidate trade-offs. Deterministic world search should remain available without AI.

**Plugins:** A future plugin contract may contribute additional searchable shared properties without being allowed to redefine core physical truth.

**Git:** Search itself creates no project history. Pinning, creating an entity, or saving a region does.

---

# 5. Layer and Authored-World Milestones

## L1. Physical and Climate Visualization Layers

### Goal

Expose the new derived fields as understandable optional map views.

### Desired layers

As features become available, Daena should be able to display layers for:

- temperature;
- precipitation;
- humidity/aridity;
- wind;
- ocean currents;
- biomes;
- storm suitability;
- earthquake hazard;
- volcanic hazard;
- tsunami exposure or individual tsunami effects;
- ice and relevant historical geography.

Layers should include understandable legends and indicate the selected epoch.

### Standalone status

**Incremental.**

Each visualization can ship with the corresponding derived field.

### Considerations

- Layers visualize derived data; toggling/editing them cannot change physical truth.
- Avoid false precision.
- Atlas may provide richer static versions of the same concepts.
- Authored layers should remain visually and conceptually distinct from generated physical layers.

### Ecosystem interaction

Layer visibility becomes useful to Find Place, Atlas export, Lore navigation, event inspection, and plugin visualization.

---

## L2. Suggested Road Routing and Persistent Roads

### Goal

Allow authors to create roads that respect the physical world instead of manually drawing arbitrary straight lines.

### Desired workflow

1. User chooses a start point.
2. User chooses an end point.
3. Daena calculates multiple plausible route suggestions.
4. The user compares them.
5. The user accepts one suggestion or cancels.
6. The accepted road becomes normal authored world data.
7. Future routing considers the accepted road network.

### Route considerations

The routing model should consider available world information such as:

- slope and cumulative climbing;
- ruggedness;
- rivers;
- lakes;
- coastlines;
- mountain passes;
- biome/traversal difficulty;
- existing roads;
- bridges or crossings when those exist later.

**Absolute elevation should not dominate route cost by itself.** A flat high plateau may be easier than repeated lower mountain ridges.

### Route alternatives

Daena should return meaningfully different suggestions rather than several nearly identical lines.

Useful strategies include:

- shortest route;
- easiest terrain;
- route maximizing use of existing roads;
- balanced route.

The UI should explain the main trade-off for each suggestion.

### Road acceptance

A suggested route does not exist in the world until the user accepts it.

After acceptance:

- it becomes a saved authored layer/object;
- it can be named and described;
- it can link to places and entities;
- future route calculations may prefer or reuse it;
- Timeline may describe its construction and later state.

### Physical-map vs Atlas detail

At physical-map resolution, Daena only needs to identify a believable route corridor.

Atlas may later refine presentation within that corridor using higher-resolution terrain detail, but must not relocate the road so far that it ceases to represent the accepted route.

### Standalone status

**Yes.**

A first iteration can use existing elevation, hydrology, and currently available climate/terrain information. Improved biomes and climate later enhance routing.

### Considerations

- Water crossings must not be treated as ordinary cheap terrain.
- Existing roads should reduce future travel cost enough to create networks naturally.
- Routing should avoid implying that a mathematically efficient path necessarily becomes a historically important road.
- Users should be free to save a less efficient route for political, religious, strategic, or narrative reasons.
- Future road classes may have different terrain tolerances.

### Ecosystem interaction

**Timeline:** Construction, destruction, closure, rebuilding, and abandonment.

**Lore:** Trade roads, pilgrimage routes, military roads, named passes, historical expeditions.

**Languages:** Roads may have names, translated names, historical names, and exonyms through normal language/entity features.

**AI:** Can explain route trade-offs or propose narrative reasons for choosing a non-optimal route. AI should not replace deterministic routing.

**Plugins:** Trade, logistics, economy, warfare, or settlement plugins may consume the accepted network.

**Atlas:** Renders roads as authored infrastructure over refined terrain.

**Git:** Accepted roads and later edits are canonical authored changes and should participate normally in project history.

---

## L3. Epoch-Aware Road History

### Goal

Make roads compatible with a world that may span roughly 200,000 years without requiring Daena to guess a universal physical lifespan for roads.

### Product decision

**Roads should not automatically expire after a fixed number of years.**

There is no useful universal rule that can decide when every trail, paved road, caravan route, imperial highway, or repeatedly rebuilt corridor ceases to exist.

### Desired road history

A road should support at least:

- construction/start date or physical offset;
- optional end/abandonment date;
- current/historical status.

Initial statuses may include:

- active;
- damaged;
- closed;
- abandoned;
- ruined.

Timeline events may explain transitions:

- road constructed;
- bridge destroyed;
- road rebuilt;
- road abandoned;
- route restored by a later state.

When viewing or routing at a particular epoch, Daena should consider only roads valid for that period and status.

### Standalone status

**Depends on L2.**

Timeline integration is strongly desirable but road data should remain valid if the Timeline module is disabled.

### Considerations

- Do not simulate material decay automatically in the first iteration.
- A later feature may distinguish a **route corridor** from a specific historical road built along it.
- A civilization may rebuild the same corridor centuries later without forcing Daena to decide whether it is philosophically the "same road."
- Long-lived roads should be authored history, not generated assumptions.

### Ecosystem interaction

This feature is especially important for Timeline, Lore, trade plugins, historical Atlas renders, migration analysis, and era-specific Find Place queries.

---

## L4. Landmass Selection for Authored Layers

### Goal

Use the physical world's known land/ocean connectivity to make large authored map layers much easier to create.

### Desired product behavior

When the user clicks land, Daena should provide an action such as:

**Select landmass**

The selection should include the entire connected landmass at the currently selected epoch.

From that selection, the user may:

- create a new authored layer;
- add it to an existing layer;
- subtract it from an existing selection;
- invert or clear the selection;
- later refine its boundary manually where the layer type permits.

### Why it is desired

Users commonly need to create large regions such as:

- countries;
- continents;
- cultural spheres;
- language regions;
- political claims;
- campaign regions;
- ecological or narrative zones.

The physical map already knows whether land cells are connected, so requiring the author to trace an island or continent manually wastes information Daena already possesses.

### Epoch awareness

Landmass selection should operate on the currently viewed epoch.

Sea-level change may:

- join islands through land bridges;
- split landmasses;
- expose continental shelves;
- submerge lowlands.

Therefore "this landmass" is partly a historical geographic question.

Creating an authored layer from the selection captures the geometry/selection the user chose; later physical epoch changes should not silently rewrite authored political or cultural borders.

### Standalone status

**Yes.**

This depends only on existing physical elevation, sea-level, and land connectivity.

It does not depend on the climate roadmap.

### Considerations

- The UI must make clear when the selected landmass is epoch-dependent.
- Authored layers created from a selection become independent world data.
- Future tools may add "select watershed," "select biome region," "select climate zone," or "select all cells matching search" using the same interaction concept.

### Ecosystem interaction

**Lore:** Quickly creates regions to link to cultures, states, peoples, or places.

**Languages:** Useful for language-area layers without coupling the physical map to the Language module.

**Timeline:** Authored borders may have their own temporal ranges independent of physical landmass history.

**Plugins:** Plugins can potentially create domain-specific regions from the selection.

**Atlas:** Renders the resulting authored layer over the physical world.

**Git:** Layer creation/editing is authored content and should be versioned normally.

---

## L5. Natural-Event Map Presentation

### Goal

Make accepted natural events easy to understand spatially without confusing them with persistent hazard fields.

### Desired product behavior

Daena should visually distinguish:

- **hazard** — long-term derived probability/rate;
- **event** — something that happened at a particular time;
- **effect area** — derived or authored consequence of that event.

Examples:

- earthquake epicenter and affected zone;
- eruption location and ash/climate influence;
- hurricane track;
- tsunami-exposed coastline.

Selecting an event on the map should allow navigation to its normal Daena entity and Timeline/Lore relationships.

### Standalone status

**Yes**, for event types that already exist.

New visualizations depend on the corresponding event feature.

### Considerations

- Events remain durable even if the underlying hazard model changes.
- The map is a spatial view of shared world history, not a separate event database.
- Disabled Timeline or Lore modules must not delete events.

---

# 6. Cross-Feature Product Capabilities

These are not required to become a single implementation subsystem. They describe behavior that several roadmap features should share.

## 6.1 Epoch-aware world inspection

At a selected coordinate and epoch, Daena should increasingly be able to answer:

- elevation;
- land/ocean status;
- slope/relief;
- temperature;
- precipitation;
- humidity/dryness;
- biome;
- nearby hydrology;
- wind;
- ocean-current context where applicable;
- hazard;
- active roads;
- linked authored entities/layers.

This capability benefits inspection, Find Place, routing, Atlas, plugins, AI context, and future simulation features.

## 6.2 Explainability

Generated recommendations should expose the world properties behind them.

Examples:

- "This road is longer but avoids two steep passes."
- "This region matches because it is warm, humid, low-elevation, and within 20 km of a perennial river."
- "This coast has high storm exposure because it lies beside warm ocean water in a supported storm-formation zone."

Daena should prefer explainable worldbuilding recommendations over opaque scores.

## 6.3 Stable materialization boundary

Generated/derived suggestions become canon only through explicit acceptance.

This rule applies to:

- roads;
- storm events;
- earthquakes/eruptions;
- tsunami events;
- AI-generated descriptions;
- suggested place locations;
- future settlement/trade suggestions.

---

# 7. Daena Ecosystem Rules

## 7.1 Timeline

Timeline should be the primary temporal view for materialized events and authored infrastructure history.

Relevant roadmap data includes:

- earthquakes;
- eruptions;
- hurricanes/storms;
- tsunamis;
- road construction;
- road damage;
- rebuilding;
- abandonment;
- climate-related historical events where the author chooses to materialize them.

Physical chronology and authored calendar chronology must remain explicitly mapped rather than silently treated as equivalent.

## 7.2 Lore

Lore consumes the consequences of the physical world; it does not own physical derivation.

Examples:

- a city linked to a hurricane;
- a religion linked to a volcanic winter;
- a kingdom linked to a road;
- a migration linked to climate change;
- a culture linked to a biome or region.

Deleting or disabling Lore must not delete the physical or historical records it references.

## 7.3 Languages

The Language module should remain loosely coupled.

Useful interactions include:

- place names;
- road names;
- regional names;
- historical names;
- exonyms/endonyms;
- names for winds, seas, storms, mountains, and geographic regions.

Physical classification should not depend on whether the Language module is enabled.

## 7.4 Plugins

Plugins should be able to build on stable, explicitly shared world information without becoming alternate owners of the physical world.

Potential future plugin use cases:

- agriculture suitability;
- trade and economy;
- ecology;
- resource placement;
- navigation;
- warfare/logistics;
- population;
- disease;
- calendars/astronomy.

A plugin may add interpretations or derived domain data, but core physical truth should remain under Daena's physical-world contract.

The product should degrade gracefully when an optional plugin is unavailable.

## 7.5 AI

AI should operate primarily as an interpretation and authoring assistant.

Good uses:

- convert "find me a rainy mountain valley near the sea" into Find Place filters;
- explain why one route is easier than another;
- summarize the likely setting consequences of a volcanic winter;
- propose names/descriptions for accepted places, events, and roads;
- help turn structured natural history into prose or Lore entries.

AI should **not**:

- invent hidden physical values;
- replace deterministic search/routing;
- silently modify canonical physical state;
- automatically turn every generated event into canon.

All physical features remain usable without AI.

## 7.6 Git and project history

Git-facing project history should emphasize meaningful authored changes.

Expected durable changes include:

- planetary configuration changes where allowed by project workflow;
- accepted/generated map replacement through explicit user action;
- authored roads;
- authored layers;
- materialized events;
- event/road Timeline changes;
- Lore/entity links.

Recomputed climate fields, search caches, Atlas render caches, route candidate scratch data, and other disposable derivations should not create meaningless project-history churn.

## 7.7 Atlas

Atlas is the high-detail presentation consumer of this roadmap.

It may use:

- temperature;
- precipitation;
- humidity/aridity;
- biomes;
- wind;
- currents;
- hydrology;
- roads;
- political/cultural layers;
- event overlays.

Atlas may add deterministic visual detail within the physical world's constraints, but it must not redefine macro geography or canonical climate/history.

---

# 8. Suggested Roadmap Order

This ordering is based on product value and dependency, not estimated engineering difficulty.

## Milestone A — World Interaction

These features can provide immediate value using current physical data:

1. **D8 — Find Place MVP**
2. **L4 — Landmass Selection**
3. **L2 — Road Routing MVP**
4. **L1 — Improved derived-field layer UX**

This milestone makes the existing physical world substantially more useful before changing its climate model.

## Milestone B — Planetary Climate Foundation

1. **G1 — Planetary and Orbital Configuration**
2. **D1 — Solar-Driven Temperature and Seasonality**
3. **D2 — Global Wind Circulation**
4. **D4 — Moisture/Humidity/Rainfall Upgrade** (SST/current evaporation, humidity, aridity, seasonal rainfall).

This establishes the causes required for richer climate.

## Milestone C — Ocean and Ecology

1. **D3 — Ocean Currents** (annual surface gyres per epoch, including large enclosed basins). D4 moisture already consumes currents; remaining consumers are D5 biomes, D6 storms, and D8 Find Place.
2. **D5 — Biome Classification**
3. Extend **Find Place** with the new fields.
4. Extend **roads** with biome/climate traversal costs.
5. Extend **Atlas** climate styles.

## Milestone D — Natural Weather and Disaster History

1. **D6 — Storm/Hurricane Suitability**
2. **D7 — Tsunami Consequences**
3. **D7 — Volcanic Climate Forcing**
4. **L5 — Event Map Presentation**
5. Deeper Timeline/Lore linking.

## Milestone E — Historical Infrastructure

1. **L3 — Epoch-Aware Road History**
2. Historical road-aware Find Place/routing.
3. Timeline-driven infrastructure state.
4. Atlas historical infrastructure rendering.

---

# 9. Explicit Non-Goals for These Milestones

Unless separately approved, this roadmap does not require:

- real-time atmospheric simulation;
- computational fluid dynamics;
- day-by-day weather for hundreds of thousands of years;
- moving tectonic plates after world acceptance;
- earthquake-driven canonical terrain deformation;
- automatic erosion after historical earthquakes;
- automatic destruction of authored settlements or entities;
- automatic expiration of roads;
- full deep-ocean circulation;
- automatic civilization, migration, economy, or population simulation;
- Atlas becoming a separate physical-world authority;
- AI-generated physics replacing deterministic derivation.

These may be revisited as independent future product proposals.

---

# 10. Future Extensions Enabled by This Roadmap

The roadmap deliberately creates useful physical context for features that are not yet part of the committed scope.

Potential future systems include:

### Settlement suggestions

Find locations based on water, climate, food potential, terrain defensibility, coast access, road access, and hazards.

### Travel and logistics

Estimate journeys based on terrain, biome, roads, rivers, season, and potentially prevailing wind/current.

### Trade networks

Use settlements, roads, rivers, sea routes, mountain passes, and resources to suggest trade corridors without automatically simulating an economy.

### Migration

Suggest plausible migration corridors from climate pressure, coastlines, rivers, passes, and existing infrastructure.

### Agriculture and ecology

Use temperature, moisture, seasonality, biome, elevation, and plugin-provided species/crop requirements.

### Watershed and climate-region selection

Extend landmass selection to automatically select:

- watershed;
- river basin;
- biome patch;
- climate zone;
- elevation band;
- Find Place result region.

### Historical route lineage

Represent the distinction between a persistent geographic corridor and the individual roads, empires, trails, and infrastructure that reuse it across tens of thousands of years.

### Natural-event consequence plugins

Allow optional systems to model population, crop, economic, military, or ecological consequences without making those simulations mandatory parts of Daena core.

---

# 11. Product Success Criteria

This roadmap succeeds when Daena's physical world becomes useful not only as a generated map, but as a coherent source of worldbuilding constraints and answers.

An author should eventually be able to:

- configure what kind of planet they are building;
- understand why regions are hot, cold, wet, dry, or storm-prone;
- view winds and ocean currents;
- obtain believable biome regions;
- search the planet for locations matching worldbuilding requirements;
- create roads that react to terrain and existing infrastructure;
- inspect those roads at the correct historical epoch;
- create whole-landmass layers with one selection;
- materialize meaningful natural events into world history;
- connect physical events to Timeline and Lore without duplicating records;
- use AI and plugins as optional consumers of the same structured world;
- render everything through Atlas without changing physical truth;
- keep Git history focused on authored world changes rather than disposable calculations.

The physical map remains the stable miniature model of the planet. Derived systems explain how that planet behaves. Authored layers and Daena's other modules describe what people, cultures, stories, and history do within it.
