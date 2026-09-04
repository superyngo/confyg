// The widget registry: one `mount` per module, keyed by the closed `Widget` vocabulary the
// core resolved (presentation §3). The record is typed `Record<Widget, Mount>`, so adding a
// Widget to the core without mapping it here is a build error rather than a blank field.
//
// Every widget honors all three Presence states (ADR 0003). A control that can express the
// Schema's default in its own value space renders an `inherited` option; one that cannot
// keeps a separate Unset affordance beside it. `filterableMenu` is never mounted in v0.1 —
// the Host clamp already turned it into `menu` — but it is mapped so the registry is total.
import type { FieldNode, Path, Widget } from "../types.js";
import { mountText } from "./text.js";
import { mountMenu } from "./menu.js";
import { mountRadio } from "./radio.js";
import { mountTristate } from "./tristate.js";
import { mountStepper } from "./stepper.js";
import { mountSlider } from "./slider.js";
import { mountDisplay } from "./display.js";

/**
 * What a widget may ask the session for. Every write is an intent, never a DOM mutation:
 * the control asks, the core decides, and the next snapshot is the only truth.
 *
 * `literal` is a **JSON literal** in every widget but `rawText`, where it is the bytes as
 * authored (design §5's Raw literal fallback).
 */
export interface Ctx {
  set(path: Path, literal: string): void;
  unset(path: Path): void;
}

export type Mount = (node: FieldNode, ctx: Ctx) => HTMLElement;

const REGISTRY: Record<Widget, Mount> = {
  text: mountText,
  rawText: mountText,
  textarea: mountText,
  masked: mountText,
  menu: mountMenu,
  filterableMenu: mountMenu,
  radio: mountRadio,
  checkboxSet: mountRadio,
  tristate: mountTristate,
  stepper: mountStepper,
  slider: mountSlider,
  displayOnly: mountDisplay,
};

export const ALL_WIDGETS = Object.keys(REGISTRY) as Widget[];

/**
 * The control for one Field. A **Locked** node and a `readOnly` one both mount as display
 * only: the Widget the core resolved says what the value *is*, and lockedness says whether
 * confyg may offer to write it (design §7).
 */
export function mount(node: FieldNode, ctx: Ctx): HTMLElement {
  if (node.meta.locked || node.meta.readOnly) return mountDisplay(node, ctx);
  return REGISTRY[node.widget](node, ctx);
}
