// Short notices in a live region; errors stay until closed.
import { h, icon } from "../dom";
import { t } from "../i18n";

type Kind = "success" | "error" | "info";

function region(): HTMLElement {
  let element = document.getElementById("toasts");
  if (!element) {
    element = h("div", { id: "toasts", class: "toasts", role: "status", "aria-live": "polite" });
    document.body.append(element);
  }
  return element;
}

export function toast(message: string, kind: Kind = "success"): void {
  const close = h(
    "button",
    { type: "button", class: "toast-close", "aria-label": t("close") },
    icon("x"),
  );
  const item = h(
    "div",
    { class: `toast ${kind}` },
    icon(kind === "error" ? "alert" : kind === "info" ? "info" : "circleCheck"),
    h("span", {}, message),
    close,
  );
  close.addEventListener("click", () => item.remove());
  region().append(item);
  if (kind !== "error") window.setTimeout(() => item.remove(), 6000);
}
