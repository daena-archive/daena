# Daena Physical World — Remaining Product Roadmap

## Status

**Type:** Product roadmap for work that is not shipped.
**Scope:** Natural-event consequences and presentation, and epoch-aware road history.
**Non-goal:** This document does not prescribe implementation architecture, algorithms, storage formats, or delivery dates.

Shipped planetary configuration, climate, biomes, storm climatology, climate views, Find Place, suggested routes, and landmass selection are the current product in [`MAPS.md`](./MAPS.md). Do not re-plan them here.

This roadmap extends that product. It must preserve the authority, authorship, detachment, persistence, offline, and recovery boundaries in [`MAPS.md`](./MAPS.md), together with the platform and AI boundaries in [`PLUGIN_PLATFORM_PLAN.md`](./PLUGIN_PLATFORM_PLAN.md) and [`AI_INTEGRATION.md`](./AI_INTEGRATION.md).

The accepted physical map remains stable. Future features must not rewrite canonical terrain because the viewed year changes or because an earthquake, eruption, storm, or other event occurs.

---

# 1. Remaining milestones

| ID  | Feature                             | Depends on                                      |
| --- | ----------------------------------- | ----------------------------------------------- |
| D7  | Extended natural-event consequences | Existing hazards; climate for volcanic forcing  |
| L5  | Natural-event map presentation      | Existing or materialized natural events         |
| L3  | Epoch-aware road history            | Accepted authored roads; Timeline               |

Suggested order: D7 and L5 can proceed from hazards and materialized events already in the product. L3 waits on authored roads, which already exist, plus an explicit validity interval. Do not block L5 on L3.

---

# 2. D7. Extended natural-event consequences

Build on existing earthquake and volcanic hazards and materialized events so a natural event can cause a physically plausible secondary effect without mutating canonical terrain.

## Tsunamis

A submarine earthquake does not automatically create a tsunami.

When an accepted earthquake can strongly displace the water column, Daena may derive a tsunami event or tsunami potential: origin area, affected coastlines, relative severity, and, if later supported, arrival ordering. Other sources may include volcanic collapse or a major submarine landslide.

## Volcanic climate forcing

A large enough eruption may create a temporary atmospheric forcing event: reduced sunlight, temporary cooling, altered precipitation, and temporary biome or climate stress. The event has a bounded duration and feeds climate history. It does not permanently change the planet.

## Earthquake consequences

An earthquake may record destructive shaking, landslides, infrastructure damage, or tsunami potential. It does not reshape the canonical physical map.

Consequences are proposals or derived facts, not silent mutations of authored entities. The author decides whether a settlement, road, state, culture, or character is affected. Severity should be explainable from the physical cause. Durable events keep their identity after later model upgrades.

Timeline is the historical record. Lore may link the event to places and peoples. A Timeline event may mark a road damaged, closed, abandoned, or rebuilt once L3 exists. AI may propose narrative from accepted facts and must not change canon. Plugins may add domain consequences without becoming mandatory.

---

# 3. L5. Natural-event map presentation

Make accepted natural events understandable on the map without confusing them with persistent hazard fields.

Distinguish:

- **hazard** — long-term derived probability or rate;
- **event** — something that happened at a particular time;
- **effect area** — a derived or authored consequence of that event.

Examples include an earthquake epicenter and affected zone, an eruption and its ash or climate influence, a hurricane track, and a tsunami-exposed coastline.

Selecting an event navigates to its normal entity and Timeline or Lore relationships. Events remain durable if the hazard model changes. The map is a view of shared world history, not a second event database. Disabling Timeline or Lore must not delete events.

---

# 4. L3. Epoch-aware road history

Accepted route suggestions already become authored road geometry. That geometry has no historical validity.

Roads must not expire after a fixed number of years. There is no universal lifespan for a trail, paved road, caravan route, or rebuilt corridor.

A road should be able to record:

- a construction or start date, or a physical offset;
- an optional end or abandonment date;
- a status such as active, damaged, closed, abandoned, or ruined.

Timeline events may explain constructed, destroyed, rebuilt, abandoned, or restored transitions. Routing and display at an epoch consider only roads valid for that period and status. Roads do not disappear because sea level or climate changed unless the author records that.

---

# 5. Not in this roadmap

Unless separately approved, do not add:

- real-time atmosphere, CFD, or day-by-day weather;
- plate motion after acceptance, earthquake terrain deformation, or automatic erosion;
- automatic destruction of settlements or automatic expiration of roads;
- deep-ocean circulation;
- civilization, migration, economy, or population simulation;
- a second physical authority in Atlas;
- AI-generated physics.

Settlement suggestions, travel time, trade networks, migration corridors, agriculture, watershed or biome selection, historical route lineage, and consequence plugins are not committed work. They may consume the current physical world later. They are not milestones of this roadmap.

---

# 6. Product rules that still apply

- Derived suggestions become canon only through explicit acceptance.
- Epoch-sensitive answers must name the epoch.
- Recommendations should expose the world properties behind them.
- Git history records authored acceptance, not disposable derivation.
- Plugins and AI consume structured world facts. They do not redefine physical truth.
