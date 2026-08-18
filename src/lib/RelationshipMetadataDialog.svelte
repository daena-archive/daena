<script lang="ts">
import { onMount, tick } from "svelte";
import type { FieldDefinition } from "../../packages/module-api/src/index";
import type { Entity, Relationship } from "$lib/project/client";

type Metadata = Record<string, unknown>;
type MetadataField = {
  key: string;
  label: string;
  type: "text" | "number" | "boolean" | "date" | "enum";
  required?: boolean | null;
  options?: string[] | null;
};
type RelationshipDefinition = FieldDefinition & { metadataFields?: MetadataField[] };

let {
  relationship,
  definition,
  entities,
  onSave,
  onClose,
}: {
  relationship: Relationship;
  definition: RelationshipDefinition | null;
  entities: Entity[];
  onSave: (metadata: Metadata) => void | Promise<void>;
  onClose: () => void;
} = $props();

let dialogElement = $state<HTMLDivElement | null>(null);
// The dialog is mounted per relationship, so initialize its editable draft once.
// svelte-ignore state_referenced_locally
let draft = $state<Metadata>(parseMetadata(relationship.metadata));
let fieldErrors = $state<Record<string, string>>({});
let saveError = $state("");
let saving = $state(false);
let lastFocused: Element | null = null;

const metadataFields = () => definition?.metadataFields ?? [];
const targetEntity = () => entities.find((entity) => entity.id === relationship.target_id);
const targetName = () => targetEntity()?.name ?? relationship.target_id;
const titleId = () => `relationship-metadata-title-${relationship.id}`;

function parseMetadata(raw: string): Metadata {
  try {
    const parsed: unknown = JSON.parse(raw || "{}");
    return typeof parsed === "object" && parsed !== null && !Array.isArray(parsed) ? (parsed as Metadata) : {};
  } catch {
    return {};
  }
}

function hasValue(value: unknown): boolean {
  return value !== undefined && value !== null && !(typeof value === "string" && value.trim() === "");
}

function valueFor(key: string): unknown {
  return draft[key];
}

function textValue(key: string): string {
  const value = valueFor(key);
  return value === undefined || value === null ? "" : String(value);
}

function dateValue(key: string): string {
  const value = valueFor(key);
  if (typeof value !== "string") return "";
  return value.length > 10 && value[10] === "T" ? value.slice(0, 10) : value;
}

function setValue(key: string, value: unknown) {
  draft = { ...draft, [key]: value };
  const nextErrors = { ...fieldErrors };
  delete nextErrors[key];
  fieldErrors = nextErrors;
  saveError = "";
}

function isValidDate(value: unknown): boolean {
  if (typeof value !== "string") return false;
  const dateOnly = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (dateOnly) {
    const [, year, month, day] = dateOnly;
    const parsed = new Date(Date.UTC(Number(year), Number(month) - 1, Number(day)));
    return (
      parsed.getUTCFullYear() === Number(year) &&
      parsed.getUTCMonth() === Number(month) - 1 &&
      parsed.getUTCDate() === Number(day)
    );
  }
  return (
    /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:\d{2})$/.test(value) &&
    !Number.isNaN(Date.parse(value))
  );
}

function invalidMessage(field: MetadataField, value: unknown): string {
  if (field.required && !hasValue(value)) return `${field.label} is required.`;
  if (!hasValue(value)) return "";
  if (field.type === "text" && typeof value !== "string") return `${field.label} must be text.`;
  if (field.type === "number" && (typeof value !== "number" || !Number.isFinite(value))) {
    return `${field.label} must be a number.`;
  }
  if (field.type === "boolean" && typeof value !== "boolean") return `${field.label} must be enabled or disabled.`;
  if (field.type === "date" && !isValidDate(value)) return `${field.label} must be a valid date.`;
  if (field.type === "enum" && !field.options?.includes(String(value))) {
    return `${field.label} must use one of the configured options.`;
  }
  return "";
}

async function submit() {
  const next = { ...draft };
  const errors: Record<string, string> = {};
  for (const field of metadataFields()) {
    const message = invalidMessage(field, next[field.key]);
    if (message) errors[field.key] = message;
    if (!hasValue(next[field.key])) delete next[field.key];
  }
  if (Object.keys(errors).length > 0) {
    fieldErrors = errors;
    return;
  }
  saving = true;
  saveError = "";
  try {
    await onSave(next);
    onClose();
  } catch (cause) {
    saveError = cause instanceof Error ? cause.message : "Could not save relationship details.";
  } finally {
    saving = false;
  }
}

function focusableElements(): HTMLElement[] {
  return Array.from(
    dialogElement?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ) ?? [],
  );
}

onMount(() => {
  lastFocused = document.activeElement;
  void tick().then(() => focusableElements()[0]?.focus());
  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = focusableElements();
    if (focusable.length === 0) {
      event.preventDefault();
      dialogElement?.focus();
      return;
    }
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };
  window.addEventListener("keydown", handleKeydown, true);
  return () => {
    window.removeEventListener("keydown", handleKeydown, true);
    if (lastFocused instanceof HTMLElement && lastFocused.isConnected) lastFocused.focus();
  };
});
</script>

<div class="relationship-metadata-backdrop" role="presentation" onclick={onClose}>
  <div
    bind:this={dialogElement}
    class="relationship-metadata-dialog"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby={titleId()}
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => event.stopPropagation()}>
    <header class="relationship-metadata-header">
      <div>
        <span class="relationship-metadata-kicker">RELATIONSHIP DETAILS</span>
        <h2 id={titleId()}>{relationship.relationship_type}</h2>
        <p>→ {targetName()}</p>
      </div>
      <button
        type="button"
        class="relationship-metadata-close"
        aria-label="Close relationship details"
        onclick={onClose}>×</button>
    </header>

    {#if metadataFields().length === 0}
      <div class="relationship-metadata-empty">
        <strong>No configurable properties for this relationship type</strong>
        <p>The relationship target is <b>{targetName()}</b>. Its metadata is managed outside this module.</p>
      </div>
      <footer class="relationship-metadata-actions">
        <button type="button" class="relationship-metadata-primary" onclick={onClose}>Close</button>
      </footer>
    {:else}
      <form
        class="relationship-metadata-form"
        onsubmit={(event) => {
          event.preventDefault();
          void submit();
        }}>
        <p class="relationship-metadata-note">
          Properties stay attached to this relationship row, so each interval can be edited independently.
        </p>
        {#each metadataFields() as field (field.key)}
          <div class="relationship-metadata-field">
            <label for={`relationship-${relationship.id}-${field.key}`}>
              <span
                >{field.label}{#if field.required}<b aria-hidden="true"> *</b>{/if}</span>
              {#if field.type === "boolean"}
                <input
                  id={`relationship-${relationship.id}-${field.key}`}
                  type="checkbox"
                  checked={valueFor(field.key) === true}
                  onchange={(event) => setValue(field.key, (event.currentTarget as HTMLInputElement).checked)} />
              {:else if field.type === "number"}
                <input
                  id={`relationship-${relationship.id}-${field.key}`}
                  type="number"
                  value={textValue(field.key)}
                  oninput={(event) => {
                    const raw = (event.currentTarget as HTMLInputElement).value;
                    setValue(field.key, raw === "" ? "" : Number(raw));
                  }} />
              {:else if field.type === "date"}
                <input
                  id={`relationship-${relationship.id}-${field.key}`}
                  type="date"
                  value={dateValue(field.key)}
                  onchange={(event) => setValue(field.key, (event.currentTarget as HTMLInputElement).value)} />
              {:else if field.type === "enum"}
                <select
                  id={`relationship-${relationship.id}-${field.key}`}
                  value={textValue(field.key)}
                  onchange={(event) => setValue(field.key, (event.currentTarget as HTMLSelectElement).value)}>
                  <option value="">Choose {field.label.toLowerCase()}</option>
                  {#each field.options ?? [] as option}<option value={option}>{option}</option>{/each}
                </select>
              {:else}
                <input
                  id={`relationship-${relationship.id}-${field.key}`}
                  type="text"
                  value={textValue(field.key)}
                  placeholder={`Add ${field.label.toLowerCase()}`}
                  oninput={(event) => setValue(field.key, (event.currentTarget as HTMLInputElement).value)} />
              {/if}
            </label>
            {#if fieldErrors[field.key]}<small class="relationship-metadata-error" role="alert"
                >{fieldErrors[field.key]}</small
              >{/if}
          </div>
        {/each}
        <p class="relationship-metadata-preservation">
          Stored properties not declared by the active schema will be preserved.
        </p>
        {#if saveError}<p class="relationship-metadata-error" role="alert">{saveError}</p>{/if}
        <footer class="relationship-metadata-actions">
          <button type="button" class="relationship-metadata-secondary" onclick={onClose}>Cancel</button>
          <button type="submit" class="relationship-metadata-primary" disabled={saving}
            >{saving ? "Saving…" : "Save details"}</button>
        </footer>
      </form>
    {/if}
  </div>
</div>

<style>
.relationship-metadata-backdrop {
  position: fixed;
  z-index: 85;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 18px;
  background: rgba(37, 37, 31, 0.32);
}
.relationship-metadata-dialog {
  width: min(500px, 100%);
  max-height: min(720px, calc(100vh - 36px));
  overflow-y: auto;
  padding: 22px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 14px;
  background: var(--surface, #fffefa);
  box-shadow: 0 24px 70px rgba(38, 42, 33, 0.25);
  outline: none;
}
.relationship-metadata-header,
.relationship-metadata-actions {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.relationship-metadata-kicker {
  display: block;
  color: var(--accent, #b4773f);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}
h2 {
  margin: 4px 0 0;
  color: var(--ink, #25251f);
  font: 700 21px/1.2 var(--font-display, Georgia, serif);
}
.relationship-metadata-header p {
  margin: 5px 0 0;
  color: var(--ink-soft, #77766d);
  font-size: 12px;
}
.relationship-metadata-close {
  width: 30px;
  height: 30px;
  flex: none;
  border: 0;
  border-radius: 7px;
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink-soft, #77766d);
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
}
.relationship-metadata-close:hover,
.relationship-metadata-close:focus-visible {
  background: #ebe6dd;
  color: var(--ink, #25251f);
  outline: 2px solid rgba(180, 119, 63, 0.2);
  outline-offset: 1px;
}
.relationship-metadata-form {
  display: grid;
  gap: 13px;
  margin-top: 20px;
}
.relationship-metadata-note,
.relationship-metadata-preservation,
.relationship-metadata-empty p {
  margin: 0;
  color: var(--ink-soft, #77766d);
  font-size: 12px;
  line-height: 1.55;
}
.relationship-metadata-field {
  display: grid;
  gap: 5px;
}
.relationship-metadata-field label {
  display: grid;
  gap: 6px;
  color: var(--ink, #25251f);
  font-size: 12px;
  font-weight: 650;
}
.relationship-metadata-field label > span {
  color: var(--ink-soft, #77766d);
  font-size: 10px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.relationship-metadata-field label > span b {
  color: var(--accent, #b4773f);
}
.relationship-metadata-field input:not([type="checkbox"]),
.relationship-metadata-field select {
  width: 100%;
  min-height: 38px;
  padding: 8px 10px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--canvas, #f7f6f2);
  color: var(--ink, #25251f);
  font-size: 13px;
  outline: none;
}
.relationship-metadata-field input:not([type="checkbox"]):focus,
.relationship-metadata-field select:focus {
  border-color: #c99965;
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.relationship-metadata-field input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: var(--accent-dark, #365342);
}
.relationship-metadata-error {
  color: #a1482f;
  font-size: 11px;
  line-height: 1.4;
}
.relationship-metadata-preservation {
  padding-top: 2px;
  font-size: 11px;
  font-style: italic;
}
.relationship-metadata-empty {
  margin-top: 22px;
  padding: 14px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 9px;
  background: var(--canvas, #f7f6f2);
}
.relationship-metadata-empty strong {
  display: block;
  margin-bottom: 5px;
  color: var(--ink, #25251f);
  font-size: 13px;
}
.relationship-metadata-actions {
  align-items: center;
  justify-content: flex-end;
  margin-top: 20px;
}
.relationship-metadata-secondary,
.relationship-metadata-primary {
  padding: 9px 14px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
}
.relationship-metadata-secondary {
  border: 1px solid var(--line, #e4e1d8);
  background: transparent;
  color: var(--ink-soft, #77766d);
}
.relationship-metadata-primary {
  border: 1px solid var(--accent-dark, #365342);
  background: var(--accent-dark, #365342);
  color: #fff;
}
.relationship-metadata-primary:disabled {
  cursor: wait;
  opacity: 0.55;
}
@media (max-width: 520px) {
  .relationship-metadata-backdrop {
    align-items: end;
    padding: 10px;
  }
  .relationship-metadata-dialog {
    max-height: calc(100vh - 20px);
    padding: 18px;
    border-radius: 12px 12px 8px 8px;
  }
}
</style>
