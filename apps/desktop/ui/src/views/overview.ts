// Overview: status tiles, next step, changes, access from the package, preview.
import { attempt, setOverview } from "../actions";
import { api } from "../api";
import { toast } from "../components/toast";
import { button, h, icon } from "../dom";
import { formatDate, t } from "../i18n";
import { state, update, whileBusy } from "../store";
import type { Overview } from "../types";

function stat(label: string, value: string, detail: string | null, tone: "ok" | "warn" | "neutral"): HTMLElement {
  return h(
    "div",
    { class: `stat ${tone}` },
    h("p", { class: "stat-label" }, label),
    h("p", { class: "stat-value" }, value),
    detail ? h("p", { class: "stat-detail" }, detail) : null,
  );
}

/** What the user should do next, derived from the project state. */
export function nextStep(project: Overview): string {
  if (!project.detected) return t("nextNoDeploy");
  if (project.packageHasCredentials && !project.storedCredentials) return t("nextTakeAccess");
  const online = project.lastUpload;
  if (online && online.target === "server" && online.created === project.current.created) {
    return t("nextUpToDate");
  }
  return t("nextPublish");
}

function onlineTile(project: Overview): HTMLElement {
  const online = project.lastUpload;
  if (!online) return stat(t("statOnline"), t("statOnlineNone"), null, "warn");
  const upToDate = online.created === project.current.created;
  const where = online.target === "folder" ? t("statOnlineFolder") : online.destination;
  return stat(
    t("statOnline"),
    online.version,
    `${where} · ${t("statOnlineSince", { date: formatDate(online.uploadedAt) })}`,
    upToDate ? "ok" : "warn",
  );
}

function accessTile(project: Overview): HTMLElement {
  if (project.storedCredentials) return stat(t("statAccess"), t("accessStored"), null, "ok");
  if (project.packageHasCredentials) return stat(t("statAccess"), t("accessInPackage"), null, "warn");
  return stat(t("statAccess"), t("accessMissing"), null, "neutral");
}

function credentialsCard(project: Overview): HTMLElement | null {
  if (!project.packageHasCredentials || project.storedCredentials) return null;
  const input = h("input", { type: "password", id: "package-password", autocomplete: "off" });
  const submit = async () => {
    const overview = await whileBusy("credentials", () =>
      attempt(() => api.importPackageCredentials(project.name, input.value)),
    );
    if (overview) {
      input.value = "";
      setOverview(overview);
      toast(t("accessStored"));
    }
  };
  return h(
    "section",
    { class: "card" },
    h("h2", {}, icon("key"), t("credentialsTitle")),
    h("p", { class: "muted" }, t("credentialsHint")),
    h("label", { class: "field", for: "package-password" }, t("packagePassword")),
    h(
      "form",
      { class: "row", onsubmit: (event: Event) => { event.preventDefault(); void submit(); } },
      input,
      h("button", { type: "submit", class: "button primary", disabled: state.busy.has("credentials") }, t("credentialsImport")),
    ),
  );
}

export function renderOverview(project: Overview): HTMLElement {
  const notice = state.notice;
  const changelog = project.current.changelog;
  return h(
    "div",
    { class: "stack" },
    notice
      ? h(
          "div",
          { class: notice.changed.length ? "banner warn" : "banner info" },
          icon(notice.changed.length ? "alert" : "info"),
          h(
            "div",
            {},
            h("p", {}, notice.message),
            notice.changed.length
              ? h("div", {}, h("p", {}, t("locallyChanged")), h("ul", { class: "small" }, notice.changed.map((path) => h("li", {}, path))))
              : null,
          ),
          h(
            "button",
            { type: "button", class: "icon-button", "aria-label": t("close"), onclick: () => update({ notice: null }) },
            icon("x"),
          ),
        )
      : null,
    h(
      "div",
      { class: "stats" },
      stat(t("statLocal"), project.current.version, formatDate(project.current.created), "neutral"),
      onlineTile(project),
      accessTile(project),
    ),
    h("div", { class: "banner next" }, icon("arrowRight"), h("p", {}, h("strong", {}, `${t("nextStep")}: `), nextStep(project))),
    credentialsCard(project),
    h(
      "section",
      { class: "card" },
      h("h2", {}, t("changes")),
      changelog.length ? h("ul", {}, changelog.map((entry) => h("li", {}, entry))) : h("p", { class: "muted" }, t("noChanges")),
      h(
        "div",
        { class: "row" },
        project.hasCompliance
          ? button(t("complianceOpen"), () => void attempt(() => api.openCompliance(project.name)), { variant: "ghost", iconName: "shield" })
          : null,
        button(t("revealFolder"), () => void attempt(() => api.revealProject(project.name)), { variant: "ghost", iconName: "folderOpen" }),
      ),
    ),
    h(
      "section",
      { class: "card" },
      h("h2", {}, icon("eye"), t("previewTitle")),
      h("p", { class: "muted" }, t("previewHint")),
      state.status.preview ? h("p", { class: "status ok" }, icon("circleCheck"), t("previewRunning", { url: state.status.preview })) : null,
    ),
  );
}
