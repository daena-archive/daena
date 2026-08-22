import type { FieldRecord, ModuleManifest } from "../../../module-api/src/index";
import { parseCalendarDate, type CalendarDate } from "../../../../src/lib/date.ts";

export type TimelineLayer = "dates" | "lifelines";
export type TimelineFieldRole = "point" | "start" | "end";

export type TimelineFieldSpec = {
  namespace: string;
  key: string;
  label: string;
  entityTypes?: readonly string[];
  role: TimelineFieldRole;
  group?: string;
  layer: TimelineLayer;
};

export type TimelineContribution = {
  id: string;
  entity: { id: string; name: string; type?: string | null };
  namespace: string;
  layer: TimelineLayer;
  label: string;
  startValue?: unknown;
  endValue?: unknown;
  startLabel?: string;
  endLabel?: string;
  pointRole?: TimelineFieldRole;
};

function appliesToEntity(spec: TimelineFieldSpec, entityType: string | null | undefined): boolean {
  return !spec.entityTypes || (entityType != null && spec.entityTypes.includes(entityType));
}

export function discoverTimelineFieldSpecs(
  manifests: readonly ModuleManifest[],
  ownerManifestId: string,
): TimelineFieldSpec[] {
  const specs: TimelineFieldSpec[] = [];
  for (const manifest of manifests) {
    if (manifest.id === ownerManifestId) continue;
    for (const schema of manifest.schemas) {
      for (const field of schema.fields) {
        if (field.type !== "date" || !field.shared) continue;
        const metadata = field.timeline;
        const role = metadata?.role ?? "point";
        if ((role === "start" || role === "end") && !metadata?.group) continue;
        specs.push({
          namespace: schema.namespace,
          key: field.key,
          label: metadata?.label?.trim() || field.label,
          entityTypes: field.entityTypes,
          role,
          group: metadata?.group ?? undefined,
          layer: metadata?.layer ?? "dates",
        });
      }
    }
  }
  return specs.sort(
    (left, right) =>
      left.namespace.localeCompare(right.namespace) ||
      (left.group ?? left.key).localeCompare(right.group ?? right.key) ||
      left.key.localeCompare(right.key),
  );
}

export function buildFieldContributions(
  entity: { id: string; name: string; type?: string | null },
  records: readonly FieldRecord[],
  specs: readonly TimelineFieldSpec[],
): TimelineContribution[] {
  const values = new Map(records.map((record) => [`${record.namespace}:${record.key}`, record.value]));
  const contributions: TimelineContribution[] = [];
  const grouped = new Map<
    string,
    {
      namespace: string;
      group: string;
      layer: TimelineLayer;
      start?: { value: unknown; label: string };
      end?: { value: unknown; label: string };
    }
  >();

  for (const spec of specs) {
    if (!appliesToEntity(spec, entity.type)) continue;
    const value = values.get(`${spec.namespace}:${spec.key}`);
    if (!parseCalendarDate(value)) continue;
    if (spec.role === "point") {
      contributions.push({
        id: `${spec.namespace}:${spec.key}:${entity.id}`,
        entity,
        namespace: spec.namespace,
        layer: spec.layer,
        label: entity.name,
        startValue: value,
        startLabel: spec.label,
        pointRole: "point",
      });
      continue;
    }

    const group = spec.group!;
    const groupKey = `${spec.namespace}:${group}:${spec.layer}`;
    const entry = grouped.get(groupKey) ?? { namespace: spec.namespace, group, layer: spec.layer };
    entry[spec.role] = { value, label: spec.label };
    grouped.set(groupKey, entry);
  }

  for (const entry of grouped.values()) {
    if (!entry.start && !entry.end) continue;
    contributions.push({
      id: `${entry.namespace}:${entry.group}:${entity.id}`,
      entity,
      namespace: entry.namespace,
      layer: entry.layer,
      label: entity.name,
      startValue: entry.start?.value ?? entry.end?.value,
      endValue: entry.start && entry.end ? entry.end.value : undefined,
      startLabel: entry.start?.label ?? entry.end?.label,
      endLabel: entry.start && entry.end ? entry.end.label : undefined,
      pointRole: entry.start && entry.end ? undefined : entry.start ? "start" : "end",
    });
  }

  return contributions.sort((left, right) => left.id.localeCompare(right.id));
}

export function timelineDateAnchor(value: unknown): { date: Date; source: CalendarDate } | null {
  const source = parseCalendarDate(value);
  if (!source) return null;
  const date = new Date(0);
  date.setUTCFullYear(source.year, (source.month ?? 1) - 1, source.day ?? 1);
  date.setUTCHours(source.hour ?? 0, source.minute ?? 0, source.second ?? 0, 0);
  return Number.isFinite(date.getTime()) ? { date, source } : null;
}
