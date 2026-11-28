// Header of a website: name, version, publisher and the two main actions.
import { togglePreview } from "../actions";
import { button, h } from "../dom";
import { formatDate, t } from "../i18n";
import { navigate } from "../router";
import { state } from "../store";
import type { Overview } from "../types";

export function renderHeader(project: Overview): HTMLElement {
  const previewRunning = Boolean(state.status.preview);
  return h(
    "header",
    { class: "page-header" },
    h(
      "div",
      {},
      h("h1", {}, project.name),
      h(
        "p",
        { class: "muted" },
        `${t("version")} ${project.current.version} · ${formatDate(project.current.created)}`,
        project.publisher ? ` · ${t("publisher")} ${project.publisher}` : "",
      ),
    ),
    h(
      "div",
      { class: "actions" },
      button(previewRunning ? t("actionPreviewStop") : t("actionPreview"), () => void togglePreview(), {
        iconName: previewRunning ? "playerStop" : "eye",
        disabled: state.busy.has("preview"),
      }),
      project.detected
        ? button(t("actionPublish"), () => navigate({ kind: "site", name: project.name, section: "publish" }), {
            variant: "primary",
            iconName: "cloudUpload",
          })
        : null,
    ),
  );
}
