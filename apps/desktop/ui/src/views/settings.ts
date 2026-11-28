// Settings: webspace (package values as placeholders), expert mappings, access.
import { attempt, setOverview } from "../actions";
import { api } from "../api";
import { confirmDialog } from "../components/dialog";
import { toast } from "../components/toast";
import { button, h, icon } from "../dom";
import { t } from "../i18n";
import { state, whileBusy } from "../store";
import type { MappingInfo, Overview, ProjectSettings } from "../types";

function optionsText(mapping: MappingInfo): string {
  const options: string[] = [];
  if (mapping.mode !== "sync") options.push(`mode=${mapping.mode}`);
  if (mapping.protect.length) options.push(`protect=${mapping.protect.join(",")}`);
  return options.length ? ` | ${options.join("; ")}` : "";
}

function emptyToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}

function field(id: string, label: string, input: HTMLElement): HTMLElement {
  return h("div", { class: "field-group" }, h("label", { class: "field", for: id }, label), input);
}

function textInput(id: string, value: string | null | undefined, placeholder: string | number | undefined, type = "text"): HTMLInputElement {
  return h("input", { id, type, value: value ?? "", placeholder: placeholder === undefined ? "" : String(placeholder) });
}

function serverForm(project: Overview): HTMLElement {
  const detected = project.detected;
  const settings = project.settings;
  const protocol = h(
    "select",
    { id: "set-protocol" },
    h("option", { value: "sftp", selected: (settings.protocol ?? detected?.protocol) === "sftp" }, "SFTP"),
    h("option", { value: "ftps", selected: (settings.protocol ?? detected?.protocol) === "ftps" }, "FTPS"),
  );
  const host = textInput("set-host", settings.host, detected?.host);
  const user = textInput("set-user", settings.user, detected?.user);
  const port = textInput("set-port", settings.port?.toString(), detected?.port, "number");
  const root = textInput("set-root", settings.remoteRoot, detected?.root);
  const domain = textInput("set-domain", settings.domain, t("domainPlaceholder"));
  const mappings = h("textarea", {
    id: "set-mappings",
    rows: 6,
    spellcheck: false,
    value: settings.mappings ?? "",
    placeholder: (detected?.mappings ?? []).map((m) => `${m.source} -> ${m.target}${optionsText(m)}`).join("\n"),
  });

  const save = async (next: ProjectSettings) => {
    const overview = await whileBusy("settings", () => attempt(() => api.saveSettings(project.name, next)));
    if (overview) {
      setOverview(overview);
      toast(t("saved"));
    }
  };

  const submit = (event: Event) => {
    event.preventDefault();
    const portValue = emptyToNull(port.value);
    const chosen = protocol.value;
    void save({
      protocol: chosen === detected?.protocol && !settings.protocol ? null : chosen,
      host: emptyToNull(host.value),
      user: emptyToNull(user.value),
      port: portValue === null ? null : Number(portValue),
      remoteRoot: emptyToNull(root.value),
      domain: emptyToNull(domain.value),
      mappings: emptyToNull(mappings.value),
    });
  };

  return h(
    "form",
    { class: "card", onsubmit: submit },
    h("h2", {}, icon("world"), t("serverTitle")),
    h("p", { class: "muted" }, t("settingsHint")),
    h(
      "div",
      { class: "form-grid" },
      field("set-protocol", t("protocol"), protocol),
      field("set-host", t("host"), host),
      field("set-user", t("user"), user),
      field("set-port", t("port"), port),
      field("set-root", t("remoteRoot"), root),
      field("set-domain", t("domain"), domain),
    ),
    h(
      "details",
      { class: "expert" },
      h("summary", {}, t("expertMappings")),
      h("p", { class: "banner warn small" }, icon("alert"), h("span", {}, t("expertWarning"))),
      mappings,
    ),
    h(
      "div",
      { class: "row" },
      h("button", { type: "submit", class: "button primary", disabled: state.busy.has("settings") }, t("save")),
      button(t("resetSettings"), () => void save({}), { variant: "ghost", disabled: state.busy.has("settings") }),
    ),
  );
}

function accessCard(project: Overview): HTMLElement {
  const password = h("input", { id: "transfer-password", type: "password", autocomplete: "off" });
  const busy = state.busy.has("access");
  const exportAccess = async () => {
    const done = await whileBusy("access", () => attempt(() => api.exportCredentials(project.name, password.value)));
    if (done) toast(t("exported"));
  };
  const importAccess = async () => {
    const overview = await whileBusy("access", () => attempt(() => api.importCredentialsFile(project.name, password.value)));
    if (overview) {
      setOverview(overview);
      toast(t("accessStored"));
    }
  };
  const forget = async () => {
    const confirmed = await confirmDialog({
      title: t("confirmForgetTitle"),
      text: t("confirmForgetText"),
      confirmLabel: t("accessForget"),
      danger: true,
    });
    if (!confirmed) return;
    const overview = await whileBusy("access", () => attempt(() => api.forgetCredentials(project.name)));
    if (overview) setOverview(overview);
  };
  return h(
    "section",
    { class: "card" },
    h("h2", {}, icon("key"), t("accessTitle")),
    h("p", { class: "muted" }, project.storedCredentials ? t("accessStoredText") : t("accessMissingText")),
    field("transfer-password", t("transferPassword"), password),
    h(
      "div",
      { class: "row" },
      project.storedCredentials ? button(t("accessExport"), () => void exportAccess(), { iconName: "download", disabled: busy }) : null,
      button(t("accessImport"), () => void importAccess(), { iconName: "upload", disabled: busy }),
      project.storedCredentials ? button(t("accessForget"), () => void forget(), { variant: "danger", iconName: "trash", disabled: busy }) : null,
    ),
  );
}

export function renderSettings(project: Overview): HTMLElement {
  return h("div", { class: "stack" }, serverForm(project), accessCard(project));
}
