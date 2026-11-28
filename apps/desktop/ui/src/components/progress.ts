// Progress display for builds and uploads: step, bar (determinate or running), details, the
// latest output line and a cancel button. Events update the element in place, so the view
// need not re-render.
import { button, h, icon } from "../dom";
import { t } from "../i18n";

export interface ProgressView {
  /** What runs now, e.g. "Step 2 of 3 · Installing dependencies". */
  title: string;
  /** 0…1, or null while the amount is unknown (running bar). */
  fraction: number | null;
  detail: string;
  line: string;
  /** How the run ended, if it did not succeed; the title then names the step. */
  ended: "failed" | "cancelled" | null;
  /** Cancel was requested and the run is stopping. */
  cancelling: boolean;
}

const views = new Map<string, ProgressView>();

function blank(): ProgressView {
  return { title: t("working"), fraction: null, detail: "", line: "", ended: null, cancelling: false };
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
  const view = progressView(id);
  // Late events must not overwrite the final state.
  if (view.ended && !("ended" in change)) return;
  views.set(id, { ...view, ...change });
  patch(id);
}

/** Marks the run as failed; the display then stays visible with the step that failed. */
export function failProgress(id: string): void {
  setProgress(id, { ended: "failed", title: t("failedAt", { step: progressView(id).title }) });
}

/** Marks the run as cancelled by the user, naming the step it stopped in. */
export function cancelProgress(id: string): void {
  // Amounts and remaining time no longer apply.
  setProgress(id, { ended: "cancelled", title: t("cancelledAt", { step: progressView(id).title }), detail: "" });
}

export function clearProgress(id: string): void {
  views.delete(id);
}

/** Whether the last run failed or was cancelled (the display then stays visible). */
export function hasEnded(id: string): boolean {
  return Boolean(views.get(id)?.ended);
}

function percent(fraction: number): number {
  return Math.round(Math.min(1, Math.max(0, fraction)) * 100);
}

function apply(root: HTMLElement, view: ProgressView): void {
  root.classList.toggle("failed", view.ended === "failed");
  root.classList.toggle("cancelled", view.ended === "cancelled");
  const title = root.querySelector<HTMLElement>(".progress-title-text");
  if (title) title.textContent = view.cancelling && !view.ended ? t("cancelling") : view.title;
  const cancel = root.querySelector<HTMLButtonElement>(".progress-cancel");
  if (cancel) {
    cancel.hidden = Boolean(view.ended);
    cancel.disabled = view.cancelling;
  }
  const bar = root.querySelector<HTMLElement>(".progress-bar");
  const fill = root.querySelector<HTMLElement>(".progress-fill");
  if (bar && fill) {
    const running = !view.ended;
    bar.classList.toggle("indeterminate", view.fraction === null && running);
    if (running && view.fraction !== null) {
      const value = percent(view.fraction);
      bar.setAttribute("aria-valuenow", String(value));
      bar.setAttribute("aria-valuetext", `${value} %`);
      // CSSOM changes are allowed by the CSP (unlike style attributes in markup).
      fill.style.width = `${value}%`;
    } else {
      bar.removeAttribute("aria-valuenow");
      bar.removeAttribute("aria-valuetext");
      fill.style.width = view.ended ? "100%" : "";
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

/**
 * Renders the display with the given id (stable across re-renders). `onCancel` adds a cancel
 * button; it may ask first and returns whether cancelling was requested.
 */
export function renderProgress(id: string, onCancel?: () => Promise<boolean>): HTMLElement {
  const cancel = onCancel
    ? button(
        t("cancel"),
        () =>
          void onCancel().then((requested) => {
            if (requested) setProgress(id, { cancelling: true });
          }),
        { variant: "ghost", iconName: "x" },
      )
    : null;
  cancel?.classList.add("progress-cancel");
  const root = h(
    "div",
    { class: "progress", id },
    h(
      "div",
      { class: "progress-head" },
      // Only the title is announced; details change every second.
      h("p", { class: "progress-title", "aria-live": "polite" }, icon("alert"), h("span", { class: "progress-title-text" })),
      cancel,
    ),
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
