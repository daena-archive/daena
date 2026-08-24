import {
  Anchor,
  Bird,
  BookOpenText,
  Boxes,
  Bug,
  Building2,
  CalendarDays,
  CalendarRange,
  Castle,
  CircleHelp,
  CloudLightning,
  Coins,
  Compass,
  Crown,
  Drama,
  Fish,
  Flag,
  Flame,
  FlaskConical,
  Flower2,
  Gem,
  Ghost,
  Hammer,
  Heart,
  Home,
  Hourglass,
  KeyRound,
  Landmark,
  Languages,
  Leaf,
  Library,
  Lightbulb,
  LockKeyhole,
  Map,
  MapPin,
  Mountain,
  Music,
  NotebookTabs,
  Package,
  Palette,
  PawPrint,
  Pickaxe,
  ScrollText,
  Shield,
  ShipWheel,
  Skull,
  Snowflake,
  Sparkles,
  Star,
  Sun,
  Moon,
  Swords,
  TentTree,
  TreePine,
  UserRound,
  UsersRound,
  WandSparkles,
  Wheat,
} from "@lucide/svelte";
import { CATALOG_ICON_IDS, type IconRef } from "../../../packages/plugin-sdk/src/generated";

export type CatalogIconId = (typeof CATALOG_ICON_IDS)[number];

export const CATALOG_ICONS = {
  agriculture: Wheat,
  anchor: Anchor,
  animal: PawPrint,
  art: Palette,
  artifact: Gem,
  bird: Bird,
  calendar: CalendarRange,
  camp: TentTree,
  castle: Castle,
  collection: Boxes,
  compass: Compass,
  concept: Lightbulb,
  craft: Hammer,
  crown: Crown,
  culture: Landmark,
  danger: Skull,
  encounter: Swords,
  era: Hourglass,
  event: CalendarDays,
  faction: Shield,
  fire: Flame,
  fish: Fish,
  flower: Flower2,
  forest: TreePine,
  group: UsersRound,
  heart: Heart,
  home: Home,
  ice: Snowflake,
  insect: Bug,
  key: KeyRound,
  language: Languages,
  library: Library,
  lock: LockKeyhole,
  magic: Sparkles,
  manuscript: BookOpenText,
  map: Map,
  mine: Pickaxe,
  moon: Moon,
  mountain: Mountain,
  music: Music,
  object: Package,
  person: UserRound,
  place: MapPin,
  plant: Leaf,
  reference: NotebookTabs,
  science: FlaskConical,
  scroll: ScrollText,
  settlement: Building2,
  ship: ShipWheel,
  spirit: Ghost,
  star: Star,
  storm: CloudLightning,
  sun: Sun,
  theatre: Drama,
  unknown: CircleHelp,
  wand: WandSparkles,
  wealth: Coins,
} as const satisfies Record<CatalogIconId, any>;

export const CATALOG_ICON_OPTIONS = CATALOG_ICON_IDS.map((id) => ({
  id,
  label: id.replace(/(^|-)([a-z])/g, (_match, _separator, letter: string) => ` ${letter.toUpperCase()}`).trim(),
  component: CATALOG_ICONS[id],
}));

export const FALLBACK_ICON: IconRef = { kind: "catalog", id: "unknown" };

export function catalogIcon(id: string) {
  return CATALOG_ICONS[id as CatalogIconId] ?? CATALOG_ICONS.unknown;
}

export function pluginIconUrl(pluginId: string, path: string): string {
  const encodedPath = path
    .split("/")
    .map((part) => encodeURIComponent(part))
    .join("/");
  return `plugin-icon://${encodeURIComponent(pluginId)}/${encodedPath}`;
}

export function userIconUrl(svg: string): string {
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

export function validateUserSvg(svg: string): string | null {
  if (!svg || new TextEncoder().encode(svg).length > 32 * 1024) return "SVG must be smaller than 32 KiB.";
  const lower = svg.toLowerCase();
  if (["<!doctype", "<!entity", "<?xml-stylesheet"].some((value) => lower.includes(value)))
    return "SVG contains unsupported markup.";
  const document = new DOMParser().parseFromString(svg, "image/svg+xml");
  if (document.querySelector("parsererror")) return "SVG could not be parsed.";
  const root = document.documentElement;
  if (root.localName !== "svg" || root.namespaceURI !== "http://www.w3.org/2000/svg")
    return "SVG needs a standard SVG root.";
  const viewBox = root
    .getAttribute("viewBox")
    ?.trim()
    .split(/[\s,]+/)
    .map(Number);
  if (
    !viewBox ||
    viewBox.length !== 4 ||
    viewBox.some((value) => !Number.isFinite(value)) ||
    viewBox[2] <= 0 ||
    viewBox[3] <= 0 ||
    viewBox[2] > 4096 ||
    viewBox[3] > 4096
  )
    return "SVG needs a valid, bounded viewBox.";
  const elements = new Set(["svg", "g", "path", "circle", "ellipse", "line", "polyline", "polygon", "rect"]);
  const attributes = new Set([
    "xmlns",
    "viewBox",
    "width",
    "height",
    "preserveAspectRatio",
    "fill",
    "fill-rule",
    "clip-rule",
    "stroke",
    "stroke-width",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-miterlimit",
    "stroke-dasharray",
    "stroke-dashoffset",
    "vector-effect",
    "opacity",
    "fill-opacity",
    "stroke-opacity",
    "transform",
    "d",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "points",
    "role",
    "aria-hidden",
    "focusable",
  ]);
  for (const element of [root, ...root.querySelectorAll("*")]) {
    if (!elements.has(element.localName)) return "SVG contains an unsupported element.";
    for (const attribute of element.attributes) {
      if (!attributes.has(attribute.name) || (attribute.namespaceURI && attribute.name !== "xmlns"))
        return "SVG contains an unsupported attribute.";
      const value = attribute.value.toLowerCase();
      if (["url(", "javascript:", "data:", "http:", "https:"].some((item) => value.includes(item)))
        return "SVG contains an external or active value.";
    }
    for (const child of element.childNodes)
      if (child.nodeType === Node.TEXT_NODE && child.textContent?.trim()) return "SVG icons cannot contain text.";
  }
  return null;
}
