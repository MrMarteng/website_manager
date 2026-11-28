// Versions kept on this computer: restore (then publish) or move to the trash.
import { attempt, setOverview } from "../actions";
import { api } from "../api";
import { confirmDialog } from "../components/dialog";
import { toast } from "../components/toast";
import { button, h } from "../dom";
import { formatDate, t } from "../i18n";
import { navigate } from "../router";
import { state, whileBusy } from "../store";
import type { Overview, VersionInfo } from "../types";
import { resetPublish } from "./publish";

async function restore(project: Overview, version: VersionInfo): Promise<void> {
  const confirmed = await confirmDialog({
    title: t("confirmRestoreTitle"),
    text: t("confirmRestoreText", { version: version.version }),
    confirmLabel: t("restore"),
  });
  if (!confirmed) return;
  const overview = await whileBusy("versions", () => attempt(() => api.restoreVersion(project.name, version.id)));
  if (!overview) return;
  resetPublish(project.name);
  setOverview(overview);
  toast(t("restored", { version: version.version }), "info");
  navigate({ kind: "site", name: project.name, section: "publish" });
}

async function remove(project: Overview, version: VersionInfo): Promise<void> {
  const confirmed = await confirmDialog({
    title: t("confirmRemoveTitle"),
    text: t("confirmRemoveText", { version: version.version }),
    confirmLabel: t("removeVersion"),
    danger: true,
  });
  if (!confirmed) return;
  const overview = await whileBusy("versions", () => attempt(() => api.removeVersion(project.name, version.id)));
  if (overview) setOverview(overview);
}

export function renderVersions(project: Overview): HTMLElement {
  const busy = state.busy.has("versions");
  const online = project.lastUpload?.created;
  return h(
    "section",
    { class: "card" },
    h("p", { class: "muted" }, t("versionsHint")),
    h(
      "ul",
      { class: "rows" },
      project.versions.map((version) =>
        h(
          "li",
          { class: "row-item" },
          h(
            "div",
            {},
            h("strong", {}, version.version),
            h("span", { class: "muted" }, ` · ${formatDate(version.created)}`),
            version.current ? h("span", { class: "badge" }, t("current")) : null,
            online === version.created ? h("span", { class: "badge ok" }, t("statOnline")) : null,
          ),
          version.current
            ? null
            : h(
                "div",
                { class: "row" },
                button(t("restore"), () => void restore(project, version), { iconName: "restore", disabled: busy }),
                button(t("removeVersion"), () => void remove(project, version), { variant: "ghost", iconName: "trash", disabled: busy }),
              ),
        ),
      ),
    ),
  );
}
