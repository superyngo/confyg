// The TypeScript mirror of the Form IR (`confyg-form/src/ir.rs`) and the one type that
// crosses the FFI boundary (`confyg-session/src/session.rs` `SetterSnapshot`). Every sum
// type is externally tagged on `kind` and every field is camelCase, exactly as the Rust
// side serializes it — so this file is a transcription, never an interpretation.

export type Seg = { Key: string } | { Index: number };
export type Path = Seg[];

// confy-core `schema::types::Violation`, consumed opaquely by the renderer: the summary
// and the field badge need its message and its keyword, nothing else.
export interface Violation {
  path: Path;
  keyword: string;
  message: string;
}

export type Widget =
  | "text"
  | "rawText"
  | "displayOnly"
  | "radio"
  | "menu"
  | "filterableMenu"
  | "checkboxSet"
  | "tristate"
  | "stepper"
  | "slider"
  | "textarea"
  | "masked";

export type Occupancy = "absent" | "empty" | "populated";

export type LockedReason = "yamlAlias" | "mergeKey";
export interface Locked {
  reason: LockedReason;
}

export type Constraint =
  | { kind: "minimum"; value: number; exclusive: boolean }
  | { kind: "maximum"; value: number; exclusive: boolean }
  | { kind: "multipleOf"; value: number }
  | { kind: "minLength"; value: number }
  | { kind: "maxLength"; value: number }
  | { kind: "pattern"; source: string }
  | { kind: "uniqueItems" };

export interface NodeMeta {
  title: string;
  description: string | null;
  violations: Violation[];
  locked: Locked | null;
  deprecated: boolean;
}

// `FieldMeta` flattens `NodeMeta`, so its fields sit at the same level.
export interface FieldMeta extends NodeMeta {
  default: unknown | null;
  examples: unknown[];
  required: boolean;
  readOnly: boolean;
  writeOnly: boolean;
  unit: string | null;
  constraints: Constraint[];
  raw: boolean;
  // Already labelled by the core: a host that re-derived these from `enum` would honor
  // `x-confyg.optionLabels` in one host and not the next.
  options: FieldOption[];
}

export interface FieldOption {
  value: unknown;
  label: string;
}

export type Presence =
  | { kind: "absent"; default: unknown | null; remarked: string | null }
  | { kind: "set"; literal: string }
  | { kind: "invalid"; literal: string; violations: Violation[] };

export interface Bounds {
  min: number | null;
  max: number | null;
}

export interface GroupToggle {
  enabled: boolean;
}

export type FieldNode = {
  kind: "field";
  path: Path;
  widget: Widget;
  intended: Widget;
  presence: Presence;
  meta: FieldMeta;
};

export type GroupNode = {
  kind: "group";
  path: Path;
  meta: NodeMeta;
  children: FormNode[];
  occupancy: Occupancy;
  toggle: GroupToggle | null;
};

export type RepeatNode = {
  kind: "repeat";
  path: Path;
  meta: NodeMeta;
  items: FormNode[];
  occupancy: Occupancy;
  bounds: Bounds;
  itemTemplate: string;
  labelFrom: string | null;
};

export type FormNode =
  | FieldNode
  | GroupNode
  | RepeatNode
  | { kind: "unknown"; path: Path; rawPreview: string }
  | { kind: "cyclic"; path: Path; schemaPtr: string };

export type Validation =
  | { kind: "available" }
  | { kind: "unavailable"; keyword: string; pointer: string };

export interface SummaryItem {
  path: Path;
  title: string;
  keyword: string;
  message: string;
}

export interface Summary {
  items: SummaryItem[];
  validation: Validation;
}

export interface Notice {
  code: string;
  message: string;
}

export interface SetterSnapshot {
  ir: FormNode;
  summary: Summary;
  text: string;
  notices: Notice[];
  fetch: { source: unknown } | null;
  canUndo: boolean;
  canRedo: boolean;
}

// A Path rendered for display and for DOM ids: `servers[0].host`.
export function pathText(path: Path): string {
  return path
    .map((seg) => ("Key" in seg ? seg.Key : `[${seg.Index}]`))
    .join(".")
    .replace(/\.\[/g, "[");
}
