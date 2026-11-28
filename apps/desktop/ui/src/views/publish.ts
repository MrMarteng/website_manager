// Publish in three steps: target and access → check changes → upload.
import { attempt, setOverview } from "../actions";
import { api, events, toApiError } from "../api";
import { confirmDialog } from "../components/dialog";
import { clearProgress, failProgress, hasFailed, renderProgress, resetProgress, setProgress } from "../components/progress";
import { toast } from "../components/toast";
import { button, h, icon } from "../dom";
import { errorText, t } from "../i18n";
import { uploadProgressView } from "../progressText";
import { state, update, whileBusy } from "../store";
import type { Overview, UploadRequest, UploadResult } from "../types";

/** Per-website form values and results, kept while switching sections. */
interface PublishState {
  kind: "server" | "localFolder";
  password: string;
  folder: string;
  acceptUnverified: boolean;
  check: UploadResult | null;
  done: UploadResult | null;
  allowOlder: boolean;
  /** Which run the progress display belongs to (shown under that step). */
  phase: "check" | "upload" | null;
}

const PROGRESS_ID = "upload-progress";

const states = new Map<string, PublishState>();

function publishState(name: string): PublishState {
  let current = states.get(name);
  if (!current) {
    current = {
      kind: "server",
      password: "",
      folder: "",
      acceptUnverified: false,
      check: null,
      done: null,
      allowOlder: false,
      phase: null,
    };
    states.set(name, current);
  }
  return current;
}

/** Forgets checks when the version changes (open, restore, rebuild). */
export function resetPublish(name: string): void {
  states.delete(name);
}

/** Feeds upload output and statistics into the progress display; called once at start-up. */
export function listenToUploadLines(): void {
  void events.onUploadLine((line) => setProgress(PROGRESS_ID, { line: line.trim() }));
  void events.onUploadProgress((progress) => setProgress(PROGRESS_ID, uploadProgressView(progress)));
}

function request(form: PublishState, dryRun: boolean): UploadRequest {
  return {
    target: form.kind === "server" ? { kind: "server", password: form.password || null } : { kind: "localFolder", folder: form.folder },
    dryRun,
    allowOlder: form.allowOlder,
    acceptUnverifiedHost: form.acceptUnverified,
  };
}

/** Runs an upload; asks once if the target has a newer version. */
async function run(project: Overview, form: PublishState, dryRun: boolean): Promise<UploadResult | undefined> {
  form.phase = dryRun ? "check" : "upload";
  resetProgress(PROGRESS_ID, dryRun ? t("progressChecking") : t("progressUploading"));
  try {
    const result = await whileBusy("upload", () => api.upload(project.name, request(form, dryRun)));
    clearProgress(PROGRESS_ID);
    form.phase = null;
    return result;
  } catch (error) {
    const apiError = toApiError(error);
    if (apiError.code === "newer-on-server" && !form.allowOlder) {
      const replace = await confirmDialog({
        title: t("newerTitle"),
        text: t("newerText", { detail: apiError.detail }),
        confirmLabel: t("newerAction"),
        danger: true,
      });
      form.phase = null;
      if (!replace) return undefined;
      form.allowOlder = true;
      return run(project, form, dryRun);
    }
    failProgress(PROGRESS_ID);
    toast(errorText(apiError), "error");
    return undefined;
  }
}

function changeList(result: UploadResult): HTMLElement | null {
  if (!result.uploadedCount && !result.deletedCount) return null;
  return h(
    "details",
    { class: "changes" },
    h("summary", {}, t("showList")),
    h(
      "ul",
      { class: "small" },
      result.deleted.map((path) => h("li", { class: "deleted" }, `− ${path}`)),
      result.uploaded.map((path) => h("li", {}, `+ ${path}`)),
    ),
  );
}

function step(number: number, title: string, active: boolean, done: boolean, ...content: (Node | null)[]): HTMLElement {
  return h(
    "section",
    { class: `card step${active ? " active" : ""}${done ? " done" : ""}` },
    h("h2", {}, h("span", { class: "step-number" }, done ? icon("check") : String(number)), title),
    ...content,
  );
}

export function renderPublish(project: Overview): HTMLElement {
  const form = publishState(project.name);
  const detected = project.detected;
  if (!detected) return h("p", { class: "muted" }, t("nextNoDeploy"));
  const busy = state.busy.has("upload");
  const rerender = () => update({});

  const radio = (kind: PublishState["kind"], label: string, hint: string | null) =>
    h(
      "label",
      { class: "choice" },
      h("input", {
        type: "radio",
        name: "target",
        value: kind,
        checked: form.kind === kind,
        onchange: () => {
          form.kind = kind;
          form.check = null;
          form.done = null;
          rerender();
        },
      }),
      h("span", {}, h("strong", {}, label), hint ? h("span", { class: "muted" }, ` ${hint}`) : null),
    );

  const needsConfirmation = form.kind === "server" && detected.protocol === "sftp" && !detected.hostKeyPinned;
  const access =
    form.kind === "server"
      ? h(
          "div",
          {},
          h("p", { class: "muted small" }, project.storedCredentials ? t("passwordStoredHint") : t("passwordMissingHint")),
          h("label", { class: "field", for: "upload-password" }, t("password")),
          h("input", {
            id: "upload-password",
            type: "password",
            autocomplete: "off",
            value: form.password,
            oninput: (event: Event) => {
              form.password = (event.target as HTMLInputElement).value;
            },
          }),
          needsConfirmation
            ? h(
                "label",
                { class: "choice warn" },
                h("input", {
                  type: "checkbox",
                  checked: form.acceptUnverified,
                  onchange: (event: Event) => {
                    form.acceptUnverified = (event.target as HTMLInputElement).checked;
                  },
                }),
                h("span", {}, t("unverified", { host: detected.host })),
              )
            : null,
        )
      : h(
          "div",
          {},
          h("label", { class: "field", for: "upload-folder" }, t("localFolder")),
          h("input", {
            id: "upload-folder",
            type: "text",
            placeholder: t("localFolderPlaceholder"),
            value: form.folder,
            oninput: (event: Event) => {
              form.folder = (event.target as HTMLInputElement).value;
            },
          }),
        );

  const check = async () => {
    form.done = null;
    form.check = (await run(project, form, true)) ?? null;
    rerender();
  };

  const upload = async () => {
    const preview = form.check;
    if (!preview) return;
    const confirmed = await confirmDialog({
      title: t("confirmUploadTitle"),
      text: t("confirmUploadText", { uploaded: preview.uploadedCount, deleted: preview.deletedCount }),
      confirmLabel: t("confirmUploadAction"),
    });
    if (!confirmed) return;
    const result = await run(project, form, false);
    if (!result) return;
    form.done = result;
    form.check = null;
    toast(t("uploadDone", { uploaded: result.uploadedCount, deleted: result.deletedCount }));
    const overview = await attempt(() => api.overview(project.name));
    if (overview) setOverview(overview);
    else rerender();
  };

  const progressFor = (phase: PublishState["phase"]) =>
    form.phase === phase && (busy || hasFailed(PROGRESS_ID)) ? renderProgress(PROGRESS_ID) : null;

  const checkResult = form.check
    ? h(
        "div",
        {},
        h(
          "p",
          {},
          form.check.uploadedCount || form.check.deletedCount
            ? t("checkResult", { uploaded: form.check.uploadedCount, deleted: form.check.deletedCount })
            : t("checkNone"),
        ),
        form.check.skippedSeed ? h("p", { class: "muted small" }, t("seedSkipped")) : null,
        changeList(form.check),
      )
    : null;

  const doneResult = form.done
    ? h(
        "div",
        { class: "status ok" },
        icon("circleCheck"),
        h("span", {}, t("uploadDone", { uploaded: form.done.uploadedCount, deleted: form.done.deletedCount })),
        project.settings.domain
          ? button(t("websiteOpen"), () => void attempt(() => api.openWebsite(project.name)), { variant: "ghost", iconName: "externalLink" })
          : null,
      )
    : null;

  return h(
    "div",
    { class: "stack" },
    step(
      1,
      t("stepTarget"),
      !form.check && !form.done,
      Boolean(form.check || form.done),
      h(
        "fieldset",
        { class: "choices" },
        h("legend", { class: "visually-hidden" }, t("stepTarget")),
        radio("server", t("targetServer"), `${detected.user}@${detected.host}${detected.root}`),
        radio("localFolder", t("targetLocal"), null),
      ),
      access,
    ),
    step(
      2,
      t("stepCheck"),
      !form.check && !form.done,
      Boolean(form.check),
      h("div", { class: "row" }, button(t("checkAction"), () => void check(), { iconName: "refresh", disabled: busy })),
      progressFor("check"),
      checkResult,
    ),
    step(
      3,
      t("stepUpload"),
      Boolean(form.check),
      Boolean(form.done),
      h(
        "div",
        { class: "row" },
        button(t("uploadAction"), () => void upload(), {
          variant: "primary",
          iconName: "cloudUpload",
          disabled: busy || !form.check || (!form.check.uploadedCount && !form.check.deletedCount),
        }),
      ),
      progressFor("upload"),
      doneResult,
    ),
  );
}
