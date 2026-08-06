# Daena × Fantasy Map Generator Integration

## Overview

Daena does not aim to replace a dedicated map editor. Instead, it integrates **Azgaar's Fantasy Map Generator (FMG)** as its geographic editing environment while providing the semantic layer that transforms a map into an interconnected world encyclopedia.

FMG is responsible for **creating and editing geography**.

Daena is responsible for **connecting geography to knowledge**.

The two applications together provide a complete worldbuilding experience where maps are no longer isolated images but become navigable gateways into the world's lore.

---

# Design Philosophy

The map is not a document.

The map is another view of the world's knowledge.

Every object visible on the map may have an associated Daena entity, and every entity may reference one or more locations on one or more maps.

Rather than duplicating FMG's functionality, Daena enriches it with relationships, history, references, and metadata.

---

# Responsibilities

## Fantasy Map Generator

Responsible for:

- Creating worlds
- Editing terrain
- Rivers
- Coastlines
- Biomes
- Elevation
- States
- Provinces
- Cultures
- Religions
- Burgs
- Roads
- Labels
- Rendering
- Procedural generation

FMG remains the authoritative editor for map geometry.

---

## Daena

Responsible for:

- World knowledge
- Articles
- References
- Relationships
- Timeline
- Search
- Cross-linking
- Metadata
- Navigation
- Multiple maps
- Story integration

Daena treats the FMG map as another data source.

---

# Geographic Entities

Any geographic object may be connected to a Daena entity.

Examples include:

- Continent
- Ocean
- Sea
- Island
- Mountain
- Volcano
- Forest
- Desert
- River
- Lake
- Kingdom
- Province
- City
- Village
- Castle
- Ruins
- Dungeon
- Road
- Trade Route

The relationship is not limited to FMG's native concepts.

Users may attach entities to arbitrary map locations.

---

# Bidirectional Navigation

Navigation should work in both directions.

## Map → Knowledge

Clicking a city opens its article.

Clicking a mountain opens its page.

Clicking a kingdom opens its description.

---

## Knowledge → Map

Opening a city article highlights it on the map.

Opening a character may show:

- birthplace
- current location
- important locations
- travel history

Opening an event may display where it occurred.

Maps become visual indexes into the world's information.

---

# Multiple Maps

A world rarely exists at a single scale.

Daena should support multiple interconnected maps.

Example hierarchy:

World

- Continent
  - Kingdom
    - Region
      - City
        - District
          - Building

Every map is independent.

Maps may reference other maps.

Examples:

- A city icon opens the city's detailed map.
- A castle opens its floor plan.
- A dungeon opens another underground map.
- A continent opens from the world map.

Navigation between maps should feel seamless.

---

# Temporal Maps

Maps represent places.

Daena represents history.

The same map may exist across different points in time.

Examples:

- Kingdom borders expand.
- Empires collapse.
- Rivers change names.
- Cities are destroyed.
- Roads appear.
- Capitals move.
- Forests disappear.

Selecting a date changes the visible world.

The map becomes another timeline visualization.

---

# Story Integration

Maps should understand stories.

Examples:

A battle event highlights:

- battlefield
- troop movements
- involved cities

A character page displays:

- birthplace
- home
- current location
- journey

An organization displays:

- headquarters
- territories
- influence

A quest displays:

- objectives
- visited locations
- completed path

The map becomes an interactive storytelling surface.

---

# Semantic Layers

Beyond geography, Daena can display additional overlays.

Examples:

Political

- Kingdoms
- Provinces
- Borders

Culture

- Languages
- Cultures
- Religions

Infrastructure

- Roads
- Ports
- Trade routes
- Fortifications

Population

- Population density
- Major settlements
- Capitals

Story

- Active quests
- Character locations
- Important events
- Battlefields

Custom

Users may define arbitrary layers.

Layers should be independently toggleable.

---

# Search

Searching should not be limited to names.

Examples:

Show:

- every dragon lair
- abandoned cities
- temples of the Moon God
- castles owned by House Raven
- locations visited by Arlen
- battles during the Third Age

Results appear both as a list and highlighted on the map.

---

# References

Every entity may reference one or more map locations.

Examples:

Character

- birthplace
- residence
- death location
- last seen

Organization

- headquarters
- territories
- outposts

Kingdom

- capital
- historical capitals
- borders

Artifact

- discovery site
- current location

Event

- location
- affected regions

One entity may exist on multiple maps.

---

# Map Independence

Maps remain portable.

An FMG map should continue functioning outside Daena.

Likewise, Daena should preserve world knowledge even if a map is temporarily unavailable.

The integration enriches both systems without tightly coupling them.

---

# Offline First

All functionality should work completely offline.

FMG is bundled with Daena.

No online services are required for:

- viewing maps
- editing maps
- navigation
- linking entities
- searching
- browsing references

Internet access should never be required for worldbuilding.

---

# Future Possibilities

Possible future extensions include:

- Multiple map providers
- Hand-drawn maps
- Historical map variants
- 3D terrain viewers
- Interactive route planning
- Distance and travel calculations
- Climate simulation
- Population simulation
- Political evolution over time
- Live synchronization between maps and timeline

The semantic layer should remain independent of any particular map editor, allowing Daena to support additional mapping tools in the future while maintaining the same knowledge graph.
