import { evaluateFormula, formulaHasCycle, parseFormula } from "./profileFormula.ts";

export const PROFILE_COLLECTION = "profile";
export const PROFILE_SCHEMA_VERSION = 1;
export const PROFILE_ALREADY_EXISTS = "This entity already has a Profile";
export const PROFILE_PRESET_ORIGINS = ["custom", "dnd", "fantasy", "scifi"] as const;

export type ProfilePresetOrigin = (typeof PROFILE_PRESET_ORIGINS)[number];

const PRESET_ORIGIN_SET = new Set<string>(PROFILE_PRESET_ORIGINS);

export const PROFILE_COMPONENT_KINDS = [
  "attribute",
  "skill",
  "proficiency",
  "trait",
  "resource",
  "condition",
  "tag",
  "derived",
] as const;

export type ProfileComponentKind = (typeof PROFILE_COMPONENT_KINDS)[number];

export const PROFILE_VALUE_TYPES = ["number", "rank", "boolean", "enum", "text", "resource"] as const;

export type ProfileValueType = (typeof PROFILE_VALUE_TYPES)[number];

export type ProfileValue =
  | { type: "number"; value: number | null }
  | { type: "rank"; value: string | null }
  | { type: "boolean"; value: boolean | null }
  | { type: "enum"; value: string | null }
  | { type: "text"; value: string | null }
  | { type: "resource"; current: number | null; max: number | null; unit: string | null };

export type ProfileComponent = {
  id: string;
  kind: ProfileComponentKind;
  name: string;
  scale?: string[];
  min?: number;
  max?: number;
  unit?: string;
  formula?: string;
  dependencies?: string[];
  override?: number;
  decimals?: number;
  value: ProfileValue;
};

export type ProfileAllocation = {
  pool: number;
};

export type ProfileDocument = {
  schemaVersion: number;
  presetOrigin?: ProfilePresetOrigin;
  allocation?: ProfileAllocation;
  components: ProfileComponent[];
};

const KIND_SET = new Set<string>(PROFILE_COMPONENT_KINDS);
const VALUE_TYPE_SET = new Set<string>(PROFILE_VALUE_TYPES);

export function canEditLoreProfile(entityType: string | null | undefined, ownerTypeIds: readonly string[]): boolean {
  return typeof entityType === "string" && ownerTypeIds.includes(entityType);
}

export function emptyProfile(): ProfileDocument {
  return { schemaVersion: PROFILE_SCHEMA_VERSION, presetOrigin: "custom", components: [] };
}

export function defaultValueForKind(kind: ProfileComponentKind): ProfileValue {
  switch (kind) {
    case "resource":
      return { type: "resource", current: null, max: null, unit: null };
    case "condition":
      return { type: "boolean", value: null };
    case "trait":
    case "tag":
      return { type: "text", value: null };
    case "derived":
      return { type: "number", value: null };
    default:
      return { type: "number", value: null };
  }
}

export function emptyValue(type: ProfileValueType): ProfileValue {
  switch (type) {
    case "resource":
      return { type: "resource", current: null, max: null, unit: null };
    case "boolean":
      return { type: "boolean", value: null };
    case "rank":
      return { type: "rank", value: null };
    case "enum":
      return { type: "enum", value: null };
    case "text":
      return { type: "text", value: null };
    default:
      return { type: "number", value: null };
  }
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function optionalFinite(value: unknown, label: string, errors: string[]): number | undefined {
  if (value === undefined) return undefined;
  if (!isFiniteNumber(value)) {
    errors.push(`${label} must be a finite number`);
    return undefined;
  }
  return value;
}

function optionalStringList(value: unknown, label: string, errors: string[]): string[] | undefined {
  if (value === undefined) return undefined;
  if (!Array.isArray(value) || value.some((entry) => typeof entry !== "string" || !entry.trim())) {
    errors.push(`${label} must be a list of names`);
    return undefined;
  }
  const trimmed = value.map((entry) => entry.trim());
  if (new Set(trimmed).size !== trimmed.length) errors.push(`${label} must be unique`);
  return trimmed;
}

export function parseProfileValue(raw: unknown, componentLabel = "Value"): ProfileValue | null {
  const errors: string[] = [];
  const value = parseValue(raw, componentLabel, errors);
  return errors.length || !value ? null : value;
}

function parseValue(raw: unknown, componentLabel: string, errors: string[]): ProfileValue | null {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    errors.push(`${componentLabel} is missing a value`);
    return null;
  }
  const value = raw as Record<string, unknown>;
  const type = value.type;
  if (typeof type !== "string" || !VALUE_TYPE_SET.has(type)) {
    errors.push(`${componentLabel} has an unknown value type`);
    return null;
  }
  if (type === "resource") {
    const current = value.current === null || value.current === undefined ? null : value.current;
    const max = value.max === null || value.max === undefined ? null : value.max;
    const unit = value.unit === null || value.unit === undefined ? null : value.unit;
    if (current !== null && !isFiniteNumber(current)) errors.push(`${componentLabel} current must be a number`);
    if (max !== null && !isFiniteNumber(max)) errors.push(`${componentLabel} max must be a number`);
    if (unit !== null && typeof unit !== "string") errors.push(`${componentLabel} unit must be text`);
    return {
      type: "resource",
      current: isFiniteNumber(current) ? current : null,
      max: isFiniteNumber(max) ? max : null,
      unit: typeof unit === "string" ? unit : null,
    };
  }
  if (type === "boolean") {
    if (value.value !== null && value.value !== undefined && typeof value.value !== "boolean") {
      errors.push(`${componentLabel} must be yes or no`);
    }
    return { type: "boolean", value: typeof value.value === "boolean" ? value.value : null };
  }
  if (type === "number") {
    if (value.value !== null && value.value !== undefined && !isFiniteNumber(value.value)) {
      errors.push(`${componentLabel} must be a number`);
    }
    return { type: "number", value: isFiniteNumber(value.value) ? value.value : null };
  }
  if (type === "rank" || type === "enum" || type === "text") {
    if (value.value !== null && value.value !== undefined && typeof value.value !== "string") {
      errors.push(`${componentLabel} must be text`);
    }
    const text = typeof value.value === "string" ? value.value : null;
    if (type === "rank") return { type: "rank", value: text };
    if (type === "enum") return { type: "enum", value: text };
    return { type: "text", value: text };
  }
  return null;
}

export function profileValidationErrors(input: unknown): string[] {
  const errors: string[] = [];
  if (!input || typeof input !== "object" || Array.isArray(input)) return ["Profile must be an object"];
  const document = input as Record<string, unknown>;
  if (document.schemaVersion !== PROFILE_SCHEMA_VERSION) errors.push("Unsupported Profile version");
  if (document.presetOrigin !== undefined && !PRESET_ORIGIN_SET.has(String(document.presetOrigin))) {
    errors.push("Unknown Profile preset");
  }
  parseAllocation(document.allocation, errors);
  if ("changes" in document) errors.push("Profile history is not stored on this record");
  if (!Array.isArray(document.components)) {
    errors.push("Profile components are required");
    return errors;
  }
  const allIds = new Set<string>();
  const byId = new Map<string, Record<string, unknown>>();
  for (const raw of document.components) {
    if (raw && typeof raw === "object" && !Array.isArray(raw) && typeof (raw as { id?: unknown }).id === "string") {
      const id = (raw as { id: string }).id.trim();
      if (id) {
        allIds.add(id);
        byId.set(id, raw as Record<string, unknown>);
      }
    }
  }
  const ids = new Set<string>();
  const formulaNodes: { id: string; dependencies: string[] }[] = [];
  document.components.forEach((raw, index) => {
    const label = `Component ${index + 1}`;
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      errors.push(`${label} is invalid`);
      return;
    }
    const component = raw as Record<string, unknown>;
    if (typeof component.id !== "string" || !component.id.trim()) errors.push(`${label} needs an id`);
    else if (ids.has(component.id)) errors.push(`Duplicate component id: ${component.id}`);
    else ids.add(component.id);
    if (typeof component.kind !== "string" || !KIND_SET.has(component.kind)) {
      errors.push(`${label} has an unknown kind`);
    }
    if (typeof component.name !== "string" || !component.name.trim()) errors.push(`${label} needs a name`);
    const scale = optionalStringList(component.scale, `${label} scale`, errors);
    const min = optionalFinite(component.min, `${label} min`, errors);
    const max = optionalFinite(component.max, `${label} max`, errors);
    if (min !== undefined && max !== undefined && min > max) errors.push(`${label} min cannot exceed max`);
    if (component.unit !== undefined && typeof component.unit !== "string") {
      errors.push(`${label} unit must be text`);
    }
    if (component.decimals !== undefined) {
      if (component.kind !== "derived") errors.push(`${label} decimals are only for derived`);
      else if (
        !Number.isInteger(component.decimals) ||
        Number(component.decimals) < 0 ||
        Number(component.decimals) > 8
      ) {
        errors.push(`${label} decimals must be an integer 0 to 8`);
      }
    }
    const value = parseValue(component.value, label, errors);
    if (value?.type === "number" && value.value !== null) {
      if (min !== undefined && value.value < min) errors.push(`${label} is below min`);
      if (max !== undefined && value.value > max) errors.push(`${label} is above max`);
    }
    if (value?.type === "resource") {
      if (min !== undefined && value.current !== null && value.current < min) {
        errors.push(`${label} is below min`);
      }
      if (max !== undefined && value.current !== null && value.current > max) {
        errors.push(`${label} is above max`);
      }
      if (value.max !== null && value.current !== null && value.current > value.max) {
        errors.push(`${label} current cannot exceed max`);
      }
    }
    if ((value?.type === "rank" || value?.type === "enum") && value.value) {
      if (!scale?.length) errors.push(`${label} needs a scale`);
      else if (!scale.includes(value.value)) errors.push(`${label} value is not on its scale`);
    }
    const formula = component.formula;
    const override = optionalFinite(component.override, `${label} override`, errors);
    if (component.kind === "derived") {
      if (typeof formula !== "string" || !formula.trim()) errors.push(`${label} needs a formula`);
      else {
        const parsed = parseFormula(formula);
        if ("error" in parsed) errors.push(`${label} ${parsed.error}`);
        else {
          const id = typeof component.id === "string" ? component.id : "";
          for (const dep of parsed.dependencies) {
            if (!allIds.has(dep)) errors.push(`${label} references unknown ${dep}`);
            else if (!isNumericRef(byId.get(dep))) errors.push(`${label} ${dep} must be numeric`);
            if (dep === id) errors.push(`${label} cannot reference itself`);
          }
          if (id) formulaNodes.push({ id, dependencies: parsed.dependencies });
        }
      }
    } else if (formula !== undefined) {
      errors.push(`${label} formula is only for derived`);
    }
    if (component.kind !== "derived" && override !== undefined) {
      errors.push(`${label} override is only for derived`);
    }
  });
  if (formulaHasCycle(formulaNodes)) errors.push("Profile formulas have a cycle");
  if (errors.length) return errors;
  const profile = readProfile(input);
  const evaluated = evaluateProfile(profile);
  for (const component of profile.components) {
    if (component.kind !== "derived") continue;
    const value = component.override ?? evaluated.get(component.id) ?? null;
    if (value === null) continue;
    const label = component.name || component.id;
    if (component.min !== undefined && value < component.min) errors.push(`${label} is below min`);
    if (component.max !== undefined && value > component.max) errors.push(`${label} is above max`);
  }
  return errors;
}

function isNumericRef(raw: Record<string, unknown> | undefined): boolean {
  if (!raw) return false;
  if (raw.kind === "derived") return true;
  const value = raw.value;
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const type = (value as { type?: unknown }).type;
  return type === "number" || type === "resource" || type === "boolean" || type === "rank" || type === "enum";
}

export function formulaDependencies(formula: string | undefined): string[] | undefined {
  if (!formula?.trim()) return undefined;
  const parsed = parseFormula(formula);
  if ("error" in parsed) return undefined;
  return parsed.dependencies;
}

export function withDerivedDependencies(component: ProfileComponent): ProfileComponent {
  const next = { ...component };
  if (next.kind !== "derived") {
    delete next.formula;
    delete next.dependencies;
    delete next.override;
    return next;
  }
  const dependencies = formulaDependencies(next.formula);
  if (dependencies) next.dependencies = dependencies;
  else delete next.dependencies;
  return next;
}

function parseAllocation(raw: unknown, errors: string[]): ProfileAllocation | undefined {
  if (raw === undefined) return undefined;
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    errors.push("Allocation must be an object");
    return undefined;
  }
  const pool = (raw as Record<string, unknown>).pool;
  if (!isFiniteNumber(pool) || pool < 0) {
    errors.push("Allocation pool must be a number 0 or greater");
    return undefined;
  }
  return { pool };
}

export function allocationSpent(profile: ProfileDocument): number {
  return profile.components.reduce((sum, component) => {
    if (
      component.kind !== "attribute" ||
      component.min === undefined ||
      component.max === undefined ||
      component.value.type !== "number" ||
      component.value.value === null
    ) {
      return sum;
    }
    return sum + component.value.value;
  }, 0);
}

export function allocationRemaining(profile: ProfileDocument): number | null {
  if (!profile.allocation) return null;
  return profile.allocation.pool - allocationSpent(profile);
}

export function parseProfile(input: unknown): ProfileDocument {
  const errors = profileValidationErrors(input);
  if (errors.length) throw new Error(errors[0]);
  return readProfile(input);
}

function readProfile(input: unknown): ProfileDocument {
  const document = input as Record<string, unknown>;
  const presetOrigin = PRESET_ORIGIN_SET.has(String(document.presetOrigin))
    ? (document.presetOrigin as ProfilePresetOrigin)
    : undefined;
  const allocation = parseAllocation(document.allocation, []);
  return {
    schemaVersion: PROFILE_SCHEMA_VERSION,
    ...(presetOrigin ? { presetOrigin } : {}),
    ...(allocation ? { allocation } : {}),
    components: (document.components as Record<string, unknown>[]).map((component) => {
      const value = parseValue(component.value, "Component", []);
      if (!value) throw new Error("Profile component is missing a value");
      const scale = Array.isArray(component.scale)
        ? component.scale.map((entry) => String(entry).trim()).filter(Boolean)
        : undefined;
      const kind = component.kind as ProfileComponentKind;
      const formula =
        kind === "derived" && typeof component.formula === "string" ? component.formula.trim() : undefined;
      const dependencies = formulaDependencies(formula);
      const override = kind === "derived" && isFiniteNumber(component.override) ? component.override : undefined;
      const decimals = Number.isInteger(component.decimals) ? (component.decimals as number) : undefined;
      return {
        id: String(component.id),
        kind,
        name: String(component.name).trim(),
        scale: scale?.length ? scale : undefined,
        min: typeof component.min === "number" ? component.min : undefined,
        max: typeof component.max === "number" ? component.max : undefined,
        unit: typeof component.unit === "string" ? component.unit : undefined,
        ...(decimals !== undefined ? { decimals } : {}),
        ...(formula ? { formula, ...(dependencies ? { dependencies } : {}) } : {}),
        ...(override !== undefined ? { override } : {}),
        value: kind === "derived" ? { type: "number", value: null } : value,
      };
    }),
  };
}

function authoredNumeric(component: ProfileComponent): number | null {
  const value = component.value;
  if (value.type === "number") return value.value;
  if (value.type === "resource") return value.current ?? value.max;
  if (value.type === "boolean") return value.value === null ? null : value.value ? 1 : 0;
  if ((value.type === "rank" || value.type === "enum") && value.value && component.scale) {
    const index = component.scale.indexOf(value.value);
    return index < 0 ? null : index;
  }
  return null;
}

export function evaluateProfile(profile: ProfileDocument): Map<string, number | null> {
  const byId = new Map(profile.components.map((component) => [component.id, component]));
  const results = new Map<string, number | null>();
  const visiting = new Set<string>();

  function numeric(component: ProfileComponent): number | null {
    if (results.has(component.id)) return results.get(component.id) ?? null;
    if (component.kind === "derived") {
      if (component.override !== undefined) {
        results.set(component.id, component.override);
        return component.override;
      }
      if (visiting.has(component.id)) {
        results.set(component.id, null);
        return null;
      }
      const parsed = parseFormula(component.formula ?? "");
      if ("error" in parsed) {
        results.set(component.id, null);
        return null;
      }
      visiting.add(component.id);
      const value = evaluateFormula(parsed.ast, (id) => {
        const target = byId.get(id);
        return target ? numeric(target) : null;
      });
      visiting.delete(component.id);
      results.set(component.id, value);
      return value;
    }
    const value = authoredNumeric(component);
    results.set(component.id, value);
    return value;
  }

  for (const component of profile.components) numeric(component);
  return results;
}

export function componentHasValue(component: ProfileComponent): boolean {
  if (component.kind === "derived") {
    return component.override !== undefined || Boolean(component.formula?.trim());
  }
  const value = component.value;
  if (value.type === "resource") {
    return value.current !== null || value.max !== null;
  }
  if (value.type === "boolean") return value.value !== null;
  if (value.type === "number") return value.value !== null;
  return Boolean(value.value?.trim());
}

export function profileCardShows(component: ProfileComponent, evaluated: number | null = null): boolean {
  if (!component.name.trim()) return false;
  if (component.kind === "trait") return true;
  if (component.kind === "derived") return Boolean(component.formula?.trim()) || component.override !== undefined;
  return componentHasValue(component);
}

export function formatProfileValue(component: ProfileComponent, evaluated: number | null = null): string {
  if (component.kind === "derived") {
    const value = component.override ?? evaluated;
    if (value == null) return "";
    const formatted = component.decimals !== undefined ? value.toFixed(component.decimals) : String(value);
    const unit = component.unit?.trim() ? ` ${component.unit.trim()}` : "";
    return `${formatted}${unit}`.trim();
  }
  const value = component.value;
  if (value.type === "resource") {
    if (value.current === null && value.max === null) return "";
    const current = value.current ?? "—";
    const max = value.max == null ? "" : ` / ${value.max}`;
    const unit = value.unit?.trim() ? ` ${value.unit.trim()}` : "";
    return `${current}${max}${unit}`.trim();
  }
  if (value.type === "boolean") {
    if (value.value === true) return "Yes";
    if (value.value === false) return "No";
    return "";
  }
  if (value.type === "number") return value.value == null ? "" : String(value.value);
  return value.value?.trim() ?? "";
}

export function newComponent(kind: ProfileComponentKind, name = ""): ProfileComponent {
  const component: ProfileComponent = {
    id:
      typeof crypto !== "undefined" && "randomUUID" in crypto
        ? crypto.randomUUID()
        : `profile-${Date.now().toString(36)}-${Math.random().toString(16).slice(2)}`,
    kind,
    name: name.trim() || kind.charAt(0).toUpperCase() + kind.slice(1),
    value: defaultValueForKind(kind),
  };
  if (kind === "derived") component.formula = "";
  return component;
}
