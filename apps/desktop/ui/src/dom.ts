// Small DOM helpers; no framework.
import { icons, type IconName } from "./icons";

type Child = Node | string | null | undefined | false;
type Props = Record<string, unknown> & { class?: string };

/** Creates an element; `on*` props become event listeners, others attributes or properties. */
export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: Props = {},
  ...children: (Child | Child[])[]
): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  for (const [key, value] of Object.entries(props)) {
    if (value === undefined || value === null || value === false) continue;
    if (key.startsWith("on") && typeof value === "function") {
      element.addEventListener(key.slice(2).toLowerCase(), value as EventListener);
    } else if (key === "class") {
      element.className = String(value);
    } else if (key in element && typeof value !== "string") {
      (element as unknown as Record<string, unknown>)[key] = value;
    } else {
      element.setAttribute(key, value === true ? "" : String(value));
    }
  }
  for (const child of children.flat()) {
    if (child === null || child === undefined || child === false) continue;
    element.append(child);
  }
  return element;
}

/** An icon as inline SVG (decorative unless a label is given). */
export function icon(name: IconName, label?: string): HTMLSpanElement {
  const wrapper = document.createElement("span");
  wrapper.className = "icon";
  // The SVG files come from the @tabler/icons package at build time, not from user input.
  wrapper.innerHTML = icons[name];
  if (label) {
    wrapper.setAttribute("role", "img");
    wrapper.setAttribute("aria-label", label);
  } else {
    wrapper.setAttribute("aria-hidden", "true");
  }
  return wrapper;
}

/** A button with optional icon. */
export function button(
  label: string,
  onClick: (event: MouseEvent) => void,
  options: { variant?: "primary" | "secondary" | "ghost" | "danger"; iconName?: IconName; disabled?: boolean } = {},
): HTMLButtonElement {
  return h(
    "button",
    {
      type: "button",
      class: `button ${options.variant ?? "secondary"}`,
      disabled: options.disabled ?? false,
      onclick: onClick,
    },
    options.iconName ? icon(options.iconName) : null,
    h("span", {}, label),
  );
}

export function clear(element: Element): void {
  element.replaceChildren();
}
