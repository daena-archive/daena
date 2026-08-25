export const VECTOR_DIAGNOSTIC_CODES = [
  "vector.source.invalid",
  "vector.source.unsupported-version",
  "vector.geometry.invalid",
  "vector.limit.exceeded",
  "vector.layer.missing",
  "vector.layer.in-use",
  "vector.generator.invalid-settings",
  "vector.renderer.unavailable",
  "asset.revision-conflict",
  "transfer.invalid",
  "transfer.expired",
] as const;

export type VectorDiagnosticCode = (typeof VECTOR_DIAGNOSTIC_CODES)[number];

export type VectorEditorStatus = "clean" | "dirty" | "saving" | "saved" | "conflict" | "error";

export type VectorEditorState = {
  status: VectorEditorStatus;
  dirty: boolean;
  conflict: boolean;
  diagnosticCode: string;
  diagnostic: string;
};

export type VectorEditorEvent =
  | { type: "loaded" }
  | { type: "geometry-changed" }
  | { type: "save-started" }
  | { type: "save-succeeded" }
  | { type: "save-conflict"; message: string }
  | { type: "save-failed"; message: string }
  | { type: "keep-editing" }
  | { type: "reload" };

export type VectorDiagnostic = {
  code: string;
  path: string | null;
  detail: string;
};

const INITIAL: VectorEditorState = {
  status: "clean",
  dirty: false,
  conflict: false,
  diagnosticCode: "",
  diagnostic: "",
};

export function initialVectorEditorState(): VectorEditorState {
  return { ...INITIAL };
}

export function parseVectorDiagnostic(message: string): VectorDiagnostic {
  const trimmed = message.trim();
  const code = VECTOR_DIAGNOSTIC_CODES.find((item) => trimmed === item || trimmed.startsWith(`${item}:`));
  if (!code) {
    if (/revision conflict/i.test(trimmed)) {
      return { code: "asset.revision-conflict", path: null, detail: trimmed };
    }
    return { code: "vector.source.invalid", path: null, detail: trimmed };
  }
  const rest = trimmed.slice(code.length).replace(/^:\s*/, "");
  const pathMatch = rest.match(/^(\$|[A-Za-z0-9_./[\]-]+):\s([\s\S]+)$/);
  if (pathMatch) return { code, path: pathMatch[1], detail: pathMatch[2] };
  return { code, path: null, detail: rest || trimmed };
}

export function formatVectorDiagnostic(parsed: VectorDiagnostic) {
  if (parsed.path) return `${parsed.code} at ${parsed.path}: ${parsed.detail}`;
  if (parsed.detail && parsed.detail !== parsed.code) return parsed.detail;
  return parsed.code;
}

export function reduceVectorEditor(state: VectorEditorState, event: VectorEditorEvent): VectorEditorState {
  switch (event.type) {
    case "loaded":
    case "reload":
      return { ...INITIAL };
    case "geometry-changed":
      if (state.conflict) return { ...state, dirty: true, status: "conflict" };
      return { ...state, dirty: true, status: "dirty", diagnostic: "", diagnosticCode: "" };
    case "save-started":
      return { ...state, status: "saving" };
    case "save-succeeded":
      return { ...INITIAL, status: "saved" };
    case "save-conflict": {
      const parsed = parseVectorDiagnostic(event.message);
      return {
        status: "conflict",
        dirty: true,
        conflict: true,
        diagnosticCode: parsed.code,
        diagnostic: formatVectorDiagnostic(parsed),
      };
    }
    case "save-failed": {
      const parsed = parseVectorDiagnostic(event.message);
      return {
        ...state,
        status: "error",
        diagnosticCode: parsed.code,
        diagnostic: formatVectorDiagnostic(parsed),
      };
    }
    case "keep-editing":
      return { ...state, conflict: false, status: state.dirty ? "dirty" : "clean", diagnostic: "", diagnosticCode: "" };
    default:
      return state;
  }
}
