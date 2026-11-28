// Progress display for builds and uploads: step, bar (determinate or running), details and the
// latest output line. Events update the element in place, so the view need not re-render.
import { h, icon } from "../dom";
import { t } from "../i18n";

export interface ProgressView {
  /** What runs now, e.g. "Step 2 of 3 · Installing dependencies". */
  title: string;
  /** 0…1, or null while the amount is unknown (running bar). */
  fraction: number | null;
  detail: string;
  line: string;
  /** Set when the run failed; the title then names the step that failed. */
  failed: boolean;
}

const views = new Map<string, ProgressView>();

function blank(): ProgressView {
  return { title: t("working"), fraction: null, detail: "", line: "", failed: false };
}

/** Current state of a progress display. */
export function progressView(id: string): ProgressView {
  return views.get(id) ?? blank();
}

/** Starts a fresh display. */
export function resetProgress(id: string, title = t("working")): void {
  views.set(id, { ...blank(), title });
  patch(id);
}

/** Changes parts of a display and updates it on screen if it is shown. */
export function setProgress(id: string, change: Partial<ProgressView>): void {
  views.set(id, { ...progressView(id), ...change });
  patch(id);
}

/** Marks the run as failed; the display then stays visible with the step that failed. */
export function failProgress(id: string): void {
  const view = progressView(id);
  setProgress(id, { failed: true, title: t("failedAt", { step: view.title }), fraction: view.fraction });
}

export function clearProgress(id: string): void {
  views.delete(id);
}

export function hasFailed(id: string): boolean {
  return views.get(id)?.failed ?? false;
}

function percent(fraction: number): number {
  return Math.round(Math.min(1, Math.max(0, fraction)) * 100);
}

function apply(root: HTMLElement, view: ProgressView): void {
  root.classList.toggle("failed", view.failed);
  const title = root.querySelector<HTMLElement>(".progress-title-text");
  if (title) title.textContent = view.title;
  const bar = root.querySelector<HTMLElement>(".progress-bar");
  const fill = root.querySelector<HTMLElement>(".progress-fill");
  if (bar && fill) {
    const determinate = view.fraction !== null && !view.failed;
    bar.classList.toggle("indeterminate", view.fraction === null && !view.failed);
    if (determinate && view.fraction !== null) {
      const value = percent(view.fraction);
      bar.setAttribute("aria-valuenow", String(value));
      bar.setAttribute("aria-valuetext", `${value} %`);
      // CSSOM changes are allowed by the CSP (unlike style attributes in markup).
      fill.style.width = `${value}%`;
    } else {
      bar.removeAttribute("aria-valuenow");
      bar.removeAttribute("aria-valuetext");
      fill.style.width = view.failed ? "100%" : "";
    }
  }
  const detail = root.querySelector<HTMLElement>(".progress-detail");
  if (detail) {
    detail.textContent = view.detail;
    detail.hidden = !view.detail;
  }
  const line = root.querySelector<HTMLElement>(".progress-line");
  if (line) line.textContent = view.line;
}

function patch(id: string): void {
  const root = document.getElementById(id);
  if (root) apply(root, progressView(id));
}

/** Renders the display with the given id (stable across re-renders). */
export function renderProgress(id: string): HTMLElement {
  const root = h(
    "div",
    { class: "progress", id },
    // Only the title is announced; details change every second.
    h("p", { class: "progress-title", "aria-live": "polite" }, icon("alert"), h("span", { class: "progress-title-text" })),
    h(
      "div",
      { class: "progress-bar", role: "progressbar", "aria-label": t("progressLabel"), "aria-valuemin": "0", "aria-valuemax": "100" },
      h("div", { class: "progress-fill" }),
    ),
    h("p", { class: "progress-detail small" }),
    h("p", { class: "progress-line small muted" }),
  );
  apply(root, progressView(id));
  return root;
}
